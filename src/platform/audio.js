// Experimental: native tray arms a specific application; normal screen selection
// still supplies video. Only that sharing session receives the native audio.
(() => {
    if (window.top !== window || location.origin !== "https://discord.com") return;
    const devices = navigator.mediaDevices;
    if (!devices?.getDisplayMedia) return;
    const getDisplayMedia = devices.getDisplayMedia.bind(devices);
    devices.getDisplayMedia = async (...args) => {
        const options = args[0];
        const audioRequested = options?.audio === true || (options?.audio && typeof options.audio === "object");
        const picker = window.Vencord?.Settings?.plugins?.WebScreenShare;
        const muted = picker?.enabled && picker.systemAudio;
        const endpoint = audioRequested && !muted ? window.__discardAudioEndpoint : null;
        const video = await getDisplayMedia(...args);
        if (!endpoint || !video.getVideoTracks().length) return video;
        let context, worker, node, timer, audio, output, rejectStart;
        let active = false;
        let intentional = false;
        let warned = false;
        let cleaned = false;
        const deadline = performance.now() + 10000;
        const startup = promise => new Promise((resolve, reject) => {
            rejectStart = reject;
            timer = setTimeout(() => reject(new Error("Native audio startup timed out")), Math.max(0, deadline - performance.now()));
            Promise.resolve(promise).then(resolve, reject);
        }).finally(() => { clearTimeout(timer); rejectStart = undefined; });
        const cleanup = () => {
            if (cleaned) return;
            cleaned = true;
            clearTimeout(timer);
            window.removeEventListener("pagehide", ended);
            worker?.terminate();
            node?.disconnect();
            output?.stream.getTracks().forEach(track => track.stop());
            if (context) void context.close().catch(error => console.error("Discard audio cleanup:", error));
        };
        const warn = text => {
            if (warned || intentional) return;
            warned = true;
            console.error("Discard native audio:", text);
            const toast = window.Vencord?.Webpack?.Common?.showToast;
            if (typeof toast === "function") {
                try { toast({ text, variant: "failure" }); return; }
                catch (error) { console.error("Discard audio notification:", error); }
            }
            // Native audio also works with Vencord disabled: retain visible errors.
            const notice = document.createElement("div");
            notice.setAttribute("role", "alert");
            notice.textContent = text;
            Object.assign(notice.style, { position: "fixed", right: "16px", bottom: "16px", maxWidth: "420px", padding: "12px", background: "#2b2d31", color: "#fff", borderRadius: "8px", zIndex: "2147483647" });
            (document.body || document.documentElement).appendChild(notice);
            setTimeout(() => notice.remove(), 10000);
        };
        const fail = error => {
            if (cleaned) return;
            rejectStart?.(error);
            cleanup();
            if (active) warn("Le son natif du partage s'est interrompu. Relance le partage pour le retablir ; la video continue.");
        };
        const ended = () => { intentional = true; rejectStart?.(new Error("Screen sharing stopped")); cleanup(); };
        for (const track of video.getVideoTracks()) {
            track.addEventListener("ended", ended, { once: true });
            const stopTrack = track.stop.bind(track);
            track.stop = () => { stopTrack(); ended(); };
        }
        window.addEventListener("pagehide", ended, { once: true });
        try {
            context = new AudioContext({ sampleRate: 48000 });
            if (context.sampleRate !== 48000) throw new Error("48 kHz audio is unavailable");
            const worklet = URL.createObjectURL(new Blob([__DISCARD_WORKLET__], { type: "application/javascript" }));
            try { await startup(context.audioWorklet.addModule(worklet)); }
            finally { URL.revokeObjectURL(worklet); }
            if (cleaned) throw new Error("Screen sharing stopped");
            node = new AudioWorkletNode(context, "discard-audio", { numberOfInputs: 0, numberOfOutputs: 1, outputChannelCount: [2] });
            node.onprocessorerror = () => fail(new Error("Native audio processor failed"));
            output = context.createMediaStreamDestination();
            node.connect(output);
            await startup(context.resume());
            if (cleaned) throw new Error("Screen sharing stopped");
            const transport = URL.createObjectURL(new Blob([__DISCARD_WORKER__], { type: "application/javascript" }));
            try { worker = new Worker(transport); }
            finally { URL.revokeObjectURL(transport); }
            await startup(new Promise((resolve, reject) => {
                worker.onmessage = ({ data }) => {
                    if (data.ready) resolve();
                    else if (data.stopped) { intentional = true; reject(new Error("Native audio disabled")); cleanup(); }
                    else if (data.error) { const error = new Error(data.error); reject(error); fail(error); }
                };
                worker.onerror = event => { const error = new Error(event.message || "Native audio worker failed"); reject(error); fail(error); };
                worker.postMessage({ endpoint, port: node.port }, [node.port]);
            }));
            if (cleaned) throw new Error("Native audio stopped");
            video.getAudioTracks().forEach(track => { video.removeTrack(track); track.stop(); });
            audio = output.stream.getAudioTracks()[0];
            video.addTrack(audio);
            // stop() does not emit ended; Discord can stop the audio track directly.
            const stopAudio = audio.stop.bind(audio);
            audio.stop = () => { if (!cleaned) intentional = true; stopAudio(); cleanup(); };
            audio.addEventListener("ended", ended, { once: true });
            active = true;
            if (video.getVideoTracks().every(track => track.readyState === "ended")) { audio.stop(); cleanup(); }
            return video;
        } catch (error) {
            cleanup();
            if (video.getVideoTracks().some(track => track.readyState === "live")) {
                // Optional WASAPI/Worklet failure must not cancel working video.
                console.error("Discard native audio unavailable:", error);
                warn("Audio natif indisponible : le partage video continue avec l'audio navigateur disponible.");
                return video;
            }
            throw error;
        }
    };
})();
