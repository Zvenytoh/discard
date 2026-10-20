// Apply a profile only after the current picker has produced a live track.
// Before that, retain upstream upper bounds so the next selection is not capped.
const share = window.__discardStream;
const screenShare = Vencord.Plugins.plugins.WebScreenShare;
const enabledShare = () => Vencord.Settings.plugins.WebScreenShare?.enabled;
const profile = () => enabledShare() && share?.track?.readyState === "live" ? share.profile : null;
const limits = selected => {
    const target = Math.round(Math.min(24000000, Math.max(2000000, 6000000 * (selected.height / 1080) ** 2 * selected.framerate / 60)));
    return { bitrateMin: 500000, bitrateTarget: target, bitrateMax: Math.round(target * 4 / 3) };
};
if (screenShare?.getGoliveMaxQuality) {
    const quality = screenShare.getGoliveMaxQuality;
    screenShare.getGoliveMaxQuality = function (...args) {
        const result = quality.apply(this, args);
        const selected = profile();
        if (selected && result) {
            const width = selected.width;
            const height = Math.min(selected.height, selected.captureHeight || selected.height);
            for (const value of [result.capture, result.encode]) {
                if (value && width && height) Object.assign(value, { width, height, framerate: selected.framerate, pixelCount: width * height });
            }
            Object.assign(result, limits(selected));
        } else if (result?.bitrateTarget === 600000) {
            result.bitrateTarget = Math.min(6000000, result.bitrateMax);
        }
        return result;
    };
}

Vencord.Webpack.onceReady.then(() => {
    if (!share) return;
    const { MediaEngineStore, UserStore, FluxDispatcher, showToast } = Vencord.Webpack.Common;
    let warned = false;
    let settingsStore;
    const warn = () => {
        if (warned) return;
        warned = true;
        showToast?.({ text: "Les modules de stream Discord ont change. Le partage navigateur reste disponible, mais le profil Discord ne peut pas etre synchronise.", variant: "failure" });
    };
    share.refresh = () => {
        const selected = profile();
        if (!selected) return;
        const connections = MediaEngineStore?.getMediaEngine?.()?.connections;
        const user = UserStore?.getCurrentUser?.()?.id;
        const values = connections?.values ? [...connections.values()] : Array.isArray(connections) ? connections : [];
        for (const connection of values) {
            if (!user || connection.streamUserId !== user) continue;
            const parameters = connection.videoStreamParameters?.[0];
            if (parameters && selected.width) {
                parameters.maxFrameRate = selected.framerate;
                parameters.maxResolution = { ...parameters.maxResolution, width: selected.width, height: Math.min(selected.height, selected.captureHeight || selected.height) };
            }
            if (!parameters && typeof connection.updateVideoQuality !== "function") warn();
            const update = connection.updateVideoQuality?.();
            update?.catch?.(error => { console.error("Discard stream refresh:", error); warn(); });
        }
    };
    share.changed = () => {
        const selected = profile();
        if (!selected) return;
        try {
            // This store may be lazy-loaded only when the share dialog opens.
            if (!settingsStore) {
                const candidate = Vencord.Webpack.findStore("ApplicationStreamingSettingsStore");
                if (typeof candidate?.initialize === "function" && typeof candidate?.getState === "function") {
                    settingsStore = candidate;
                    settingsStore.addChangeListener?.(fromDiscord);
                }
            }
            const store = settingsStore;
            if (typeof store?.initialize !== "function" || typeof store?.getState !== "function") warn();
            else {
                const state = store.getState();
                store.initialize({ ...state, resolution: selected.height, fps: selected.framerate, preset: 3 });
                store.emitChange?.();
            }
        } catch (error) { console.error("Discard stream settings:", error); warn(); }
        share.refresh();
    };
    const fromDiscord = () => {
        const selected = profile();
        if (!selected) return;
        const state = settingsStore.getState();
        const height = Number(state.resolution), framerate = Number(state.fps);
        if (![480, 720, 1080, 1440, 2160].includes(height) || ![15, 30, 60, 120].includes(framerate)) return;
        if (selected.height === height && selected.framerate === framerate) return;
        const track = share.track;
        Object.assign(selected, { height, framerate });
        // Discord's own stream menu must update capture too, not only its label.
        void track.applyConstraints({ ...track.getConstraints(), height: { ideal: height, max: height }, frameRate: { ideal: framerate, max: framerate } }).then(() => {
            if (share.track !== track || track.readyState !== "live") return;
            const actual = track.getSettings();
            Object.assign(selected, { width: actual.width, captureHeight: actual.height });
            share.refresh();
        }).catch(error => {
            console.error("Discard stream constraints:", error);
            showToast?.({ text: "Le navigateur ne peut pas appliquer ce profil de partage. Le partage actuel continue.", variant: "failure" });
        });
    };
    FluxDispatcher.subscribe("STREAM_CREATE", share.refresh);
    share.changed();
}).catch(error => console.error("Discard stream integration:", error));

// Native-to-page activity only; no page-to-native commands.
Vencord.Webpack.onceReady.then(() => {
    const empty = {};
    let previous = empty;
    let running = false;
    const update = () => {
        let activity = window.__discardActivity ?? null;
        if (activity && Object.values(window.__discardRpc ?? empty).some(rpc => rpc.application_id === activity.application_id)) activity = null;
        Vencord.Webpack.Common.FluxDispatcher.dispatch({ type: "LOCAL_ACTIVITY_UPDATE", socketId: "discard-game-detection", activity });
    };
    const rpc = async () => {
        if (running) return;
        running = true;
        try {
            // Serialize asset/app lookups so disconnect cannot be overtaken by
            // an older update that was still awaiting Discord's metadata.
            do {
                const current = window.__discardRpc ?? empty;
                const plugin = Vencord.Plugins.plugins["WebRichPresence (arRPC)"];
                if (!Vencord.Settings.plugins["WebRichPresence (arRPC)"]?.enabled || typeof plugin?.handleEvent !== "function") {
                    previous = empty;
                    return;
                }
                for (const socketId of new Set([...Object.keys(previous), ...Object.keys(current)])) {
                    const activity = current[socketId] ?? null;
                    if (JSON.stringify(previous[socketId]) === JSON.stringify(current[socketId])) continue;
                    try { await plugin.handleEvent({ data: JSON.stringify({ socketId, activity }) }); }
                    catch (error) { console.error("Discard RPC:", error); }
                }
                previous = current;
            } while (previous !== (window.__discardRpc ?? empty));
        } finally { running = false; update(); }
    };
    window.addEventListener("discard-activity", update);
    window.addEventListener("discard-rpc", rpc);
    void rpc();
    update();
}).catch(error => console.error("Discard game activity:", error));
