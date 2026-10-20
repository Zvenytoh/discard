// Experimental: native tray arms a specific application; normal screen selection
// still supplies video. Only that sharing session receives the native audio.
(() => {
    if (window.top !== window || location.origin !== "https://discord.com") return;
    const devices = navigator.mediaDevices;
    if (!devices?.getDisplayMedia) return;
    const getDisplayMedia = devices.getDisplayMedia.bind(devices);
    devices.getDisplayMedia = async (...args) => {
        const endpoint = window.__discardAudioEndpoint;
        const video = await getDisplayMedia(...args);
        if (!endpoint || !video.getVideoTracks().length) return video;
        let context, socket, node, timer, audio, rejectStart;
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
            socket?.close();
            node?.disconnect();
            if (context) void context.close().catch(error => console.error("Discard audio cleanup:", error));
        };
        const ended = () => { audio?.stop(); rejectStart?.(new Error("Screen sharing stopped")); cleanup(); };
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
            const output = context.createMediaStreamDestination();
            node.connect(output);
            await startup(context.resume());
            if (cleaned) throw new Error("Screen sharing stopped");
            await startup(new Promise((resolve, reject) => {
                socket = new WebSocket(endpoint);
                socket.binaryType = "arraybuffer";
                socket.onmessage = ({ data }) => {
                    if (data === "ready") { resolve(); }
                    else if (typeof data === "string" && data.startsWith("error:")) { reject(new Error(data.slice(6))); cleanup(); }
                    else if (data instanceof ArrayBuffer && data.byteLength <= 65536 && data.byteLength % 8 === 0) node.port.postMessage(data, [data]);
                };
                socket.onerror = () => reject(new Error("Native audio connection failed"));
                socket.onclose = () => { reject(new Error("Native audio stopped")); output.stream.getTracks().forEach(track => track.stop()); cleanup(); };
            }));
            video.getAudioTracks().forEach(track => { video.removeTrack(track); track.stop(); });
            audio = output.stream.getAudioTracks()[0];
            video.addTrack(audio);
            // stop() does not emit ended; Discord can stop the audio track directly.
            const stopAudio = audio.stop.bind(audio);
            audio.stop = () => { stopAudio(); cleanup(); };
            audio.addEventListener("ended", cleanup, { once: true });
            if (video.getVideoTracks().every(track => track.readyState === "ended")) { audio.stop(); cleanup(); }
            return video;
        } catch (error) {
            cleanup();
            if (video.getVideoTracks().some(track => track.readyState === "live")) {
                // Optional WASAPI/Worklet failure must not cancel working video.
                console.error("Discard native audio unavailable:", error);
                window.Vencord?.Webpack?.Common?.showToast?.({ text: "Audio natif indisponible : le partage video continue avec l'audio navigateur disponible.", variant: "failure" });
                return video;
            }
            throw error;
        }
    };
})();
