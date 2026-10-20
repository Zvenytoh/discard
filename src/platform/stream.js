// Observe existing browser media, without changing codecs or capture constraints.
(() => {
    if (window.top !== window || location.origin !== "https://discord.com") return;
    const peers = new Set();
    const PeerConnection = window.RTCPeerConnection;
    if (PeerConnection) window.RTCPeerConnection = new Proxy(PeerConnection, {
        construct(target, args, newTarget) {
            const peer = Reflect.construct(target, args, newTarget);
            peers.add(peer);
            peer.addEventListener("connectionstatechange", () => {
                if (peer.connectionState === "closed") peers.delete(peer);
            });
            return peer;
        }
    });
    window.__discardStreamDiagnostic = async () => {
        const lines = [];
        const tracks = new Set();
        for (const peer of peers) {
            for (const sender of peer.getSenders()) {
                if (sender.track?.kind !== "video" || sender.track.readyState !== "live") continue;
                const track = sender.track;
                if (!tracks.has(track)) {
                    tracks.add(track);
                    const settings = track.getSettings();
                    lines.push(`${settings.displaySurface ? "Partage" : "Video/camera"} - capture : ${settings.width ?? "?"} x ${settings.height ?? "?"}, ${settings.frameRate ?? "?"} FPS`);
                }
                try {
                    const stats = await sender.getStats();
                    stats.forEach(report => {
                        if (report.type !== "outbound-rtp" || report.isRemote || report.kind === "audio") return;
                        lines.push(`Encodage : ${report.frameWidth ?? "?"} x ${report.frameHeight ?? "?"}, ${report.framesPerSecond ?? "?"} FPS ; limitation : ${report.qualityLimitationReason ?? "inconnue"}`);
                    });
                } catch { lines.push("Statistiques d'encodage indisponibles."); }
            }
        }
        alert(lines.length ? lines.join("\n") + "\n\nValeurs locales ; ne mesurent pas la qualite recue par les autres participants." : "Aucun flux video WebRTC actif accessible.");
    };
})();
