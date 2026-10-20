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
