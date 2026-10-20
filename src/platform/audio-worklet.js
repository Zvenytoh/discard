// Bounded 48 kHz stereo PCM queue. Never connect this output to the speakers.
class DiscardAudio extends AudioWorkletProcessor {
    constructor() {
        super();
        this.samples = new Float32Array(16384);
        this.read = 0;
        this.count = 0;
        this.port.onmessage = ({ data }) => {
            const input = new Float32Array(data);
            if (input.length > this.samples.length || input.length % 2) return;
            const overflow = Math.max(0, this.count + input.length - this.samples.length);
            this.read = (this.read + overflow) % this.samples.length;
            this.count -= overflow;
            for (let i = 0; i < input.length; i++) {
                this.samples[(this.read + this.count + i) % this.samples.length] = Number.isFinite(input[i]) ? input[i] : 0;
            }
            this.count += input.length;
        };
    }
    process(_, outputs) {
        const [left, right] = outputs[0];
        for (let i = 0; i < left.length; i++) {
            if (this.count >= 2) {
                left[i] = this.samples[this.read];
                right[i] = this.samples[(this.read + 1) % this.samples.length];
                this.read = (this.read + 2) % this.samples.length;
                this.count -= 2;
            } else { left[i] = right[i] = 0; }
        }
        return true;
    }
}
registerProcessor("discard-audio", DiscardAudio);
