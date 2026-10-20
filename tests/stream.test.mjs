// Run explicitly with: node --test tests/stream.test.mjs
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import vm from "node:vm";

const source = file => readFileSync(new URL(`../${file}`, import.meta.url), "utf8");
const browser = capture => {
    const context = { navigator: { mediaDevices: { getDisplayMedia: capture } }, location: { origin: "https://discord.com" }, console, setTimeout, clearTimeout };
    context.window = context;
    context.top = context;
    context.addEventListener = () => {};
    return vm.createContext(context);
};

test("video-only, omitted audio and Vencord mute never start native capture", async () => {
    const stream = { getVideoTracks: () => [{}] };
    for (const [audio, muted] of [[false, false], [undefined, false], [{}, true]]) {
        const context = browser(async () => stream);
        context.__discardAudioEndpoint = "ws://127.0.0.1:1234/token";
        context.Vencord = { Settings: { plugins: { WebScreenShare: { enabled: true, systemAudio: muted } } } };
        context.AudioContext = class { constructor() { assert.fail("Native audio must stay off"); } };
        vm.runInContext(source("src/platform/audio.js"), context);
        assert.equal(await context.navigator.mediaDevices.getDisplayMedia({ video: true, audio }), stream);
    }
});

test("audio Worker drops old packets and retains only the latest pending packet", () => {
    const sent = [], events = [];
    let socket;
    const port = { postMessage: data => sent.push(data), close() {} };
    const context = vm.createContext({
        ArrayBuffer, DataView,
        Date: { now: () => 1000000 },
        performance: { timeOrigin: 1000000, now: () => 0 },
        self: { postMessage: data => events.push(data), close() {} },
        WebSocket: class { constructor() { socket = this; } close() {} }
    });
    vm.runInContext(source("src/platform/audio-worker.js"), context);
    context.self.onmessage({ data: { port, endpoint: "ws://127.0.0.1:1234/token" } });
    const packet = (age, sample) => {
        const data = new ArrayBuffer(16);
        new DataView(data).setFloat64(0, 1000000 - age, true);
        new Float32Array(data, 8).fill(sample);
        return { data };
    };
    socket.onmessage(packet(251, 0));
    assert.equal(sent.length, 0);
    socket.onmessage(packet(0, 1));
    socket.onmessage(packet(0, 2));
    socket.onmessage(packet(0, 3));
    assert.equal(sent.length, 1);
    port.onmessage();
    assert.equal(sent.length, 2);
    assert.equal(new Float32Array(sent[1])[0], 3);
    socket.onclose({ code: 1000, reason: "disabled" });
    assert.equal(events.at(-1).stopped, true);
});

test("post-start native audio failure warns once and keeps video alive", async () => {
    const tracks = [], warnings = [];
    let worker;
    const video = { readyState: "live", stop() { this.readyState = "ended"; }, addEventListener() {} };
    const audio = { stop() { this.stopped = true; }, addEventListener() {} };
    const stream = { getVideoTracks: () => [video], getAudioTracks: () => tracks, addTrack: track => tracks.push(track) };
    const context = browser(async () => stream);
    Object.assign(context, {
        __discardAudioEndpoint: "ws://127.0.0.1:1234/token",
        performance: { now: () => 0 },
        URL: { createObjectURL: () => "blob:worker", revokeObjectURL() {} },
        Blob: class {},
        removeEventListener() {},
        AudioContext: class {
            sampleRate = 48000;
            audioWorklet = { addModule: async () => {} };
            resume() { return Promise.resolve(); }
            close() { return Promise.resolve(); }
            createMediaStreamDestination() { return { stream: { getTracks: () => [audio], getAudioTracks: () => [audio] } }; }
        },
        AudioWorkletNode: class { port = {}; connect() {} disconnect() {} },
        Worker: class {
            constructor() { worker = this; }
            postMessage() { this.onmessage({ data: { ready: true } }); }
            terminate() { this.terminated = true; }
        },
        Vencord: { Webpack: { Common: { showToast: toast => warnings.push(toast) } } }
    });
    const script = source("src/platform/audio.js").replace("__DISCARD_WORKLET__", '"stub"').replace("__DISCARD_WORKER__", '"stub"');
    vm.runInContext(script, context);
    assert.equal(await context.navigator.mediaDevices.getDisplayMedia({ video: true, audio: true }), stream);
    worker.onmessage({ data: { error: "capture failed" } });
    worker.onmessage({ data: { error: "duplicate close" } });
    assert.equal(warnings.length, 1);
    assert.equal(video.readyState, "live");
    assert.equal(audio.stopped, true);
    assert.equal(worker.terminated, true);
});

test("picker profile reaches Discord settings and encoding without capping the next share", async () => {
    let settings = { resolution: 1080, fps: 30, soundshareEnabled: true };
    let listener;
    const context = browser(async options => {
        const track = {
            readyState: "live", stop() {}, addEventListener() {},
            getSettings: () => ({ width: Math.round(options.video.height.max * 21 / 9), height: options.video.height.max })
        };
        return { getVideoTracks: () => [track] };
    });
    const quality = () => ({ capture: {}, encode: {}, bitrateTarget: 600000, bitrateMax: 8000000 });
    const store = { getState: () => settings, initialize: value => { settings = value; }, emitChange() {}, addChangeListener: fn => { listener = fn; } };
    context.Vencord = {
        Settings: { plugins: { WebScreenShare: { enabled: true }, "WebRichPresence (arRPC)": { enabled: false } } },
        Plugins: { plugins: { WebScreenShare: { getGoliveMaxQuality: quality } } },
        Webpack: { onceReady: Promise.resolve(), findStore: () => store, Common: {
            FluxDispatcher: { subscribe() {}, dispatch() {} },
            MediaEngineStore: { getMediaEngine: () => ({ connections: new Set() }) },
            UserStore: { getCurrentUser: () => ({ id: "local" }) }
        } }
    };
    vm.runInContext(source("src/stream.js"), context);
    vm.runInContext(source("src/mods.js"), context);
    await Promise.resolve();
    for (const height of [720, 1440]) {
        const stream = await context.navigator.mediaDevices.getDisplayMedia({ video: { height, frameRate: 60 }, audio: false });
        const result = context.Vencord.Plugins.plugins.WebScreenShare.getGoliveMaxQuality();
        assert.equal(settings.resolution, height);
        assert.equal(settings.fps, 60);
        assert.equal(settings.soundshareEnabled, true);
        assert.equal(result.encode.height, height);
        assert.equal(result.encode.framerate, 60);
        assert.equal(result.encode.width, Math.round(height * 21 / 9));
        stream.getVideoTracks()[0].stop();
    }
    assert.equal(typeof listener, "function");
});
