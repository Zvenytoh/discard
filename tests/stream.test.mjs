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
    assert.equal(new Float32Array(sent[1].samples)[0], 3);
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

const streamFixture = async (apply = async () => {}) => {
    let settings = { resolution: 1080, fps: 30, soundshareEnabled: true }, listener, available = true;
    let height = 720, fps = 60;
    const warnings = [];
    const track = {
        readyState: "live", stop() { this.readyState = "ended"; }, addEventListener() {},
        getSettings: () => ({ width: Math.round(height * 21 / 9), height }),
        getConstraints: () => ({}),
        applyConstraints: async constraints => { await apply(constraints); height = constraints.height.max; fps = constraints.frameRate.max; }
    };
    const context = browser(async options => { height = options.video.height.max; return { getVideoTracks: () => [track] }; });
    const quality = () => ({ capture: {}, encode: {}, bitrateTarget: 600000, bitrateMax: 8000000 });
    const store = { getState: () => settings, initialize: value => { settings = value; }, emitChange: () => listener?.(), addChangeListener: fn => { listener = fn; } };
    context.Vencord = {
        Settings: { plugins: { WebScreenShare: { enabled: true }, "WebRichPresence (arRPC)": { enabled: false } } },
        Plugins: { plugins: { WebScreenShare: { getGoliveMaxQuality: quality } } },
        Webpack: { onceReady: Promise.resolve(), findStore: () => available ? store : null, Common: {
            showToast: value => warnings.push(value),
            FluxDispatcher: { subscribe() {}, dispatch() {} },
            MediaEngineStore: { getMediaEngine: () => ({ connections: new Set() }) },
            UserStore: { getCurrentUser: () => ({ id: "local" }) }
        } }
    };
    vm.runInContext(source("src/stream.js"), context);
    vm.runInContext(source("src/mods.js"), context);
    await Promise.resolve();
    return {
        context, track, warnings,
        state: () => settings,
        available: value => { available = value; },
        select: async (height, fps = 60) => context.navigator.mediaDevices.getDisplayMedia({ video: { height, frameRate: fps }, audio: false }),
        change: (height, fps = 60) => { settings = { ...settings, resolution: height, fps }; listener(); }
    };
};
const settled = () => new Promise(resolve => setImmediate(resolve));

test("picker profile reaches Discord settings and encoding without capping the next share", async () => {
    const fixture = await streamFixture();
    for (const height of [720, 1440]) {
        fixture.track.readyState = "live";
        const stream = await fixture.select(height);
        const result = fixture.context.Vencord.Plugins.plugins.WebScreenShare.getGoliveMaxQuality();
        assert.equal(fixture.state().resolution, height);
        assert.equal(fixture.state().fps, 60);
        assert.equal(fixture.state().soundshareEnabled, true);
        assert.equal(result.encode.height, height);
        assert.equal(result.encode.framerate, 60);
        assert.equal(result.encode.width, Math.round(height * 21 / 9));
        stream.getVideoTracks()[0].stop();
    }
});

test("a rejected quality change restores the last accepted Discord and picker profile", async () => {
    const fixture = await streamFixture(async () => { throw new Error("unsupported profile"); });
    await fixture.select(720);
    fixture.change(2160, 120);
    await settled();
    assert.equal(fixture.state().resolution, 720);
    assert.equal(fixture.state().fps, 60);
    assert.equal(fixture.context.Vencord.Settings.plugins.WebScreenShare.resolution, "720");
    assert.equal(fixture.context.__discardStream.profile.height, 720);
    assert.equal(fixture.warnings.length, 1);
    assert.equal(fixture.track.readyState, "live");
});

test("rapid changes serialize, coalesce and roll back to the most recent success", async () => {
    const operations = [];
    const fixture = await streamFixture(constraints => new Promise((resolve, reject) => operations.push({ height: constraints.height.max, resolve, reject })));
    await fixture.select(720);
    fixture.change(1440);
    fixture.change(2160);
    fixture.change(1080);
    assert.equal(operations.length, 1);
    operations[0].resolve();
    await settled();
    assert.deepEqual(operations.map(value => value.height), [1440, 1080]);
    assert.equal(fixture.state().resolution, 1080);
    operations[1].reject(new Error("latest rejected"));
    await settled();
    assert.equal(fixture.state().resolution, 1440);
    assert.equal(fixture.context.__discardStream.profile.height, 1440);
});

test("missing private modules preserve browser capture and retry next share", async () => {
    const fixture = await streamFixture();
    fixture.available(false);
    const stream = await fixture.select(720);
    assert.equal(stream.getVideoTracks()[0].readyState, "live");
    assert.equal(fixture.warnings.length, 1);
    fixture.available(true);
    await fixture.select(1440);
    assert.equal(fixture.state().resolution, 1440);
});

const workletFixture = () => {
    let processor;
    const context = vm.createContext({ sampleRate: 48000, currentTime: 0, ArrayBuffer, Float32Array,
        AudioWorkletProcessor: class { port = { postMessage() {} }; },
        registerProcessor: (_, value) => { processor = value; }
    });
    vm.runInContext(source("src/platform/audio-worklet.js"), context);
    return { context, audio: new processor() };
};
const audioPacket = (capturedAt, value = 1, ageMs = 20) => ({ data: { samples: new Float32Array(2048).fill(value).buffer, capturedAt, ageMs } });

test("worklet waits for playout time and resets after a missing packet", () => {
    const { context, audio } = workletFixture();
    const left = new Float32Array(128), right = new Float32Array(128);
    audio.port.onmessage(audioPacket(1000000));
    audio.process([], [[left, right]]);
    assert.ok(left.every(value => value === 0));
    context.currentTime = 0.02;
    audio.process([], [[left, right]]);
    assert.ok(left.every(value => value === 1));
    audio.port.onmessage(audioPacket(1000100, 2));
    context.currentTime = 0.04;
    audio.process([], [[left, right]]);
    assert.ok(left.every(value => value === 2));
});

test("worklet estimates drift and keeps its queue and correction bounded", () => {
    const { context, audio } = workletFixture();
    for (let i = 0; i < 120; i++) {
        context.currentTime = i * 1024 / 48000;
        audio.port.onmessage(audioPacket(1000000 + i * 1024 / 48.0048));
    }
    assert.ok(audio.sourceFramesPerMs > 48);
    assert.ok(audio.sourceFramesPerMs <= 48 * 1.005);
    assert.ok(audio.count >= 0 && audio.count <= audio.samples.length);
    const output = [new Float32Array(256), new Float32Array(256)];
    audio.process([], [output]);
    assert.ok(output.every(channel => channel.every(Number.isFinite)));
});

test("small negative clock error resamples without inserting block-boundary silence", () => {
    const { context, audio } = workletFixture();
    const output = [new Float32Array(128), new Float32Array(128)];
    audio.port.onmessage(audioPacket(1000000));
    audio.sourceFramesPerMs = 47.99;
    context.currentTime = 0.02;
    audio.process([], [output]);
    context.currentTime += 128 / 48000;
    audio.process([], [output]);
    assert.ok(output.every(channel => channel.every(value => value === 1)));
});
