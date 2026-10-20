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
        // A page dialog remains visible after tray restore and does not rely on
        // browser alert handling. Show progress before awaiting any WebRTC API.
        document.getElementById("discard-stream-diagnostic")?.remove();
        const dialog = document.createElement("dialog");
        dialog.id = "discard-stream-diagnostic";
        dialog.style.cssText = "background:#242429;color:#eee;border:1px solid #555;border-radius:8px;padding:20px;max-width:90vw";
        const title = document.createElement("h3");
        title.textContent = "Diagnostic du stream";
        const report = document.createElement("textarea");
        report.readOnly = true;
        report.style.cssText = "display:block;width:650px;max-width:75vw;height:220px;background:#121214;color:#eee;padding:10px";
        report.value = "Collecte des statistiques...";
        const close = document.createElement("button");
        close.textContent = "Fermer";
        close.onclick = () => { dialog.close(); dialog.remove(); };
        dialog.addEventListener("cancel", () => dialog.remove(), { once: true });
        dialog.append(title, report, close);
        document.body.append(dialog);
        dialog.showModal();
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
                    let timeout;
                    const stats = await Promise.race([
                        sender.getStats(),
                        new Promise((_, reject) => { timeout = setTimeout(() => reject(new Error("Stats timeout")), 3000); })
                    ]).finally(() => clearTimeout(timeout));
                    stats.forEach(report => {
                        if (report.type !== "outbound-rtp" || report.isRemote || report.kind === "audio") return;
                        lines.push(`Encodage : ${report.frameWidth ?? "?"} x ${report.frameHeight ?? "?"}, ${report.framesPerSecond ?? "?"} FPS ; limitation : ${report.qualityLimitationReason ?? "inconnue"}`);
                    });
                } catch { lines.push("Statistiques d'encodage indisponibles."); }
            }
        }
        report.value = lines.length ? lines.join("\n") + "\n\nValeurs locales ; ne mesurent pas la qualite recue par les autres participants." : "Aucun flux video WebRTC actif accessible. Verifie que le stream est demarre dans cette fenetre.";
        report.focus();
        report.select();
    };
})();
