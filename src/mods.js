// Apply a profile only after the current picker has produced a live track.
// Before that, retain upstream upper bounds so the next selection is not capped.
const share = window.__discardStream;
const screenShare = Vencord.Plugins.plugins.WebScreenShare;
const enabledShare = () => Vencord.Settings.plugins.WebScreenShare?.enabled;
const profile = () => {
    if (!enabledShare() || share?.track?.readyState !== "live" || !share.profile) return null;
    // A captured window can resize or switch surfaces without replacing its track.
    const actual = share.track.getSettings();
    if (actual.width > 0 && actual.height > 0) {
        share.profile.width = actual.width;
        share.profile.captureHeight = actual.height;
    }
    return share.profile;
};
const limits = selected => {
    const height = Math.min(selected.height, selected.captureHeight || selected.height);
    const pixels = (selected.width || height * 16 / 9) * height;
    const target = Math.round(Math.min(24000000, Math.max(2000000, 6000000 * pixels / (1920 * 1080) * selected.framerate / 60)));
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
    let pending, applying = false, publishing = false;
    const warn = () => {
        if (warned) return;
        warned = true;
        showToast?.({ text: "Les modules de stream Discord ont change. Le partage navigateur reste disponible, mais le profil Discord ne peut pas etre synchronise.", variant: "failure" });
    };
    const refresh = () => {
        const selected = profile();
        if (!selected) return;
        const connections = MediaEngineStore?.getMediaEngine?.()?.connections;
        const user = UserStore?.getCurrentUser?.()?.id;
        if (!user || (!Array.isArray(connections) && typeof connections?.values !== "function")) { warn(); return; }
        const values = typeof connections.values === "function" ? [...connections.values()] : connections;
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
    share.refresh = () => {
        try { if (profile() && share.check()) refresh(); }
        catch (error) { console.error("Discard stream refresh:", error); warn(); }
    };
    share.check = (starting = false) => {
        if (starting) warned = false;
        if (!enabledShare()) return false;
        try {
            const fixes = Vencord.Settings.plugins.WebScreenShareFixes;
            if (typeof screenShare?.getGoliveMaxQuality !== "function" ||
                (fixes?.enabled && !Vencord.Plugins.plugins.WebScreenShareFixes) ||
                typeof MediaEngineStore?.getMediaEngine !== "function" ||
                typeof UserStore?.getCurrentUser !== "function") { warn(); return false; }
            // Resolve per share: Discord may lazy-load or replace this store.
            const candidate = Vencord.Webpack.findStore("ApplicationStreamingSettingsStore");
            if (!candidate && starting) return false;
            if (typeof candidate?.initialize !== "function" || typeof candidate?.getState !== "function" ||
                typeof candidate?.addChangeListener !== "function") { warn(); return false; }
            if (settingsStore !== candidate) {
                settingsStore?.removeChangeListener?.(fromDiscord);
                settingsStore = candidate;
                settingsStore.addChangeListener(fromDiscord);
            }
            return true;
        } catch (error) { console.error("Discard stream compatibility:", error); warn(); return false; }
    };
    share.changed = () => {
        const selected = profile();
        if (!selected || !share.check()) return;
        publishing = true;
        try {
            const state = settingsStore.getState();
            settingsStore.initialize({ ...state, resolution: selected.height, fps: selected.framerate, preset: 3 });
            const picker = Vencord.Settings.plugins.WebScreenShare;
            picker.resolution = String(selected.height);
            picker.frameRate = String(selected.framerate);
            settingsStore.emitChange?.();
        } catch (error) { console.error("Discard stream settings:", error); warn(); }
        finally { publishing = false; }
        share.refresh();
    };
    const applyPending = async () => {
        if (applying) return;
        applying = true;
        try {
            while (pending) {
                const request = pending;
                pending = undefined;
                const { track, height, framerate } = request;
                if (share.track !== track || track.readyState !== "live" || !profile()) continue;
                const previous = share.profile;
                try {
                    await track.applyConstraints({ ...track.getConstraints(), height: { ideal: height, max: height }, frameRate: { ideal: framerate, max: framerate } });
                    if (share.track !== track || track.readyState !== "live") continue;
                    const actual = track.getSettings();
                    share.profile = { height, framerate, width: actual.width, captureHeight: actual.height };
                    // Commit successful capture, but don't publish an obsolete request.
                    if (!pending) share.changed();
                } catch (error) {
                    if (share.track !== track || track.readyState !== "live" || pending) continue;
                    share.profile = previous;
                    share.changed();
                    console.error("Discard stream constraints:", error);
                    showToast?.({ text: "Ce profil n'est pas disponible. Le dernier profil valide a ete retabli.", variant: "failure" });
                }
            }
        } finally { applying = false; }
    };
    const fromDiscord = () => {
        try {
            if (publishing) return;
            const selected = profile();
            if (!selected) return;
            const state = settingsStore.getState();
            const height = Number(state.resolution), framerate = Number(state.fps);
            if (![480, 720, 1080, 1440, 2160].includes(height) || ![15, 30, 60, 120].includes(framerate)) return;
            if (!applying && selected.height === height && selected.framerate === framerate) return;
            pending = { track: share.track, height, framerate };
            void applyPending().catch(error => { console.error("Discard stream profile:", error); warn(); });
        } catch (error) { console.error("Discard stream settings:", error); warn(); }
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
