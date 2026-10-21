// Bounded timestamped stereo PCM. Output goes only to the shared audio track.
class DiscardAudio extends AudioWorkletProcessor {
    constructor() {
        super();
        this.samples = new Float32Array(16384);
        this.read = this.count = this.phase = 0;
        this.captureStart = null;
        this.readTime = this.renderStart = this.nextTime = 0;
        this.framesPerMs = sampleRate / 1000;
        this.sourceFramesPerMs = this.framesPerMs;
        this.clockTime = this.clockFrames = 0;
        this.port.onmessage = ({ data }) => {
            try {
                if (!data || !(data.samples instanceof ArrayBuffer) || data.samples.byteLength % 8 || !Number.isFinite(data.capturedAt) || !Number.isFinite(data.ageMs)) return;
                const input = new Float32Array(data.samples);
                if (!input.length || input.length > this.samples.length || input.length % 2) return;
                if (this.captureStart !== null && data.capturedAt < this.nextTime - 5) return;
                if (this.captureStart === null || Math.abs(data.capturedAt - this.nextTime) > 5) {
                    // Missing packets are a timeline gap, not contiguous faster audio.
                    this.read = this.count = this.phase = 0;
                    this.captureStart = data.capturedAt;
                    this.renderStart = currentTime + (40 - Math.max(0, data.ageMs)) / 1000;
                    this.clockTime = data.capturedAt;
                    this.clockFrames = 0;
                    this.sourceFramesPerMs = this.framesPerMs;
                } else if (data.capturedAt - this.clockTime >= 1000) {
                    // Estimate the source sample clock over seconds, not packet jitter.
                    const measured = this.clockFrames / (data.capturedAt - this.clockTime);
                    const bounded = Math.max(this.framesPerMs * 0.995, Math.min(this.framesPerMs * 1.005, measured));
                    this.sourceFramesPerMs = this.sourceFramesPerMs * 0.9 + bounded * 0.1;
                    this.clockTime = data.capturedAt;
                    this.clockFrames = 0;
                }
                this.clockFrames += input.length / 2;
                if (!this.count) { this.readTime = data.capturedAt; this.phase = 0; }
                this.nextTime = data.capturedAt + input.length / 2 / this.sourceFramesPerMs;
                const overflow = Math.max(0, this.count + input.length - this.samples.length);
                this.advance(overflow / 2);
                for (let i = 0; i < input.length; i++) {
                    this.samples[(this.read + this.count + i) % this.samples.length] = Number.isFinite(input[i]) ? input[i] : 0;
                }
                this.count += input.length;
            } finally { this.port.postMessage(null); }
        };
    }
    advance(frames) {
        this.read = (this.read + frames * 2) % this.samples.length;
        this.count -= frames * 2;
        this.readTime += frames / this.sourceFramesPerMs;
    }
    process(_, outputs) {
        const [left, right] = outputs[0];
        left.fill(0); right.fill(0);
        if (!this.count || this.captureStart === null) return true;
        const desired = this.captureStart + (currentTime - this.renderStart) * 1000;
        let error = desired - this.readTime - this.phase / this.sourceFramesPerMs;
        if (error > 10) {
            this.advance(Math.min(this.count / 2, Math.floor(error * this.sourceFramesPerMs)));
            this.phase = 0;
            error = desired - this.readTime;
        }
        // Small clock differences use fractional interpolation; large stalls drop old data.
        const rate = 1 + Math.max(-0.005, Math.min(0.005, error * 0.0001));
        const waiting = desired < this.captureStart || error < -10;
        const start = waiting ? Math.min(left.length, Math.max(0, Math.ceil(-error * this.framesPerMs))) : 0;
        for (let i = start; i < left.length && this.count >= 2; i++) {
            const next = this.count >= 4 ? (this.read + 2) % this.samples.length : this.read;
            left[i] = this.samples[this.read] + (this.samples[next] - this.samples[this.read]) * this.phase;
            right[i] = this.samples[(this.read + 1) % this.samples.length] + (this.samples[(next + 1) % this.samples.length] - this.samples[(this.read + 1) % this.samples.length]) * this.phase;
            this.phase += rate;
            const frames = Math.min(this.count / 2, Math.floor(this.phase));
            this.advance(frames);
            this.phase -= frames;
            if (!this.count) this.phase = 0;
        }
        return true;
    }
}
registerProcessor("discard-audio", DiscardAudio);
