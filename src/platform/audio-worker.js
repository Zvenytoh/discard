// Dedicated transport: Discord's UI thread must not relay each audio packet.
let socket, port, pending;
let inFlight = false;
let closed = false;
const epoch = Date.now(), started = performance.now();
const fail = message => {
    if (closed) return;
    closed = true;
    self.postMessage({ error: message });
    socket?.close();
    port?.close();
    self.close();
};
const fresh = data => {
    // Monotonic epoch time survives a wall-clock adjustment during the share.
    const age = epoch + performance.now() - started - new DataView(data).getFloat64(0, true);
    return age >= -100 && age <= 250;
};
const post = data => {
    if (!fresh(data)) return;
    inFlight = true;
    const samples = data.slice(8);
    port.postMessage(samples, [samples]);
};
self.onmessage = ({ data }) => {
    if (socket || closed) return;
    port = data.port;
    port.onmessage = () => {
        inFlight = false;
        if (pending) { const next = pending; pending = undefined; post(next); }
    };
    socket = new WebSocket(data.endpoint);
    socket.binaryType = "arraybuffer";
    socket.onmessage = ({ data }) => {
        if (data === "ready") self.postMessage({ ready: true });
        else if (typeof data === "string" && data.startsWith("error:")) fail(data.slice(6));
        else if (data instanceof ArrayBuffer && data.byteLength > 8 && data.byteLength <= 65544 && data.byteLength % 8 === 0 && fresh(data)) {
            if (inFlight) pending = data;
            else post(data);
        }
    };
    socket.onerror = () => fail("Native audio connection failed");
    socket.onclose = event => {
        if (event.code === 1000 && event.reason === "disabled") {
            closed = true;
            self.postMessage({ stopped: true });
            port.close();
            self.close();
        } else fail("Native audio stopped");
    };
};
