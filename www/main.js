import init, { QrStreamDecoder, ScanStatus } from "./pkg/cube.js";

let decoder = null;
let stream = null;
let animationId = null;

const video = document.getElementById("video");
const canvas = document.getElementById("canvas");
const ctx = canvas.getContext("2d", { willReadFrequently: true });
const startBtn = document.getElementById("start-btn");
const stopBtn = document.getElementById("stop-btn");
const scanLine = document.getElementById("scan-line");
const statusDiv = document.getElementById("status");
const progressFill = document.getElementById("progress-fill");
const downloadArea = document.getElementById("download-area");
const downloadBtn = document.getElementById("download-btn");

async function run() {
    try {
        await init();
        console.log("Wasm module loaded");
        statusDiv.firstChild.textContent = "Ready to scan.";
        startBtn.onclick = startCamera;
        stopBtn.onclick = stopCamera;
    } catch (e) {
        console.error("Failed to load wasm:", e);
        statusDiv.firstChild.textContent = "Error loading Wasm module.";
    }
}

async function startCamera() {
    try {
        decoder = new QrStreamDecoder();

        // Prefer rear camera
        const constraints = {
            video: {
                facingMode: "environment",
                width: { ideal: 1280 },
                height: { ideal: 720 },
            },
        };

        stream = await navigator.mediaDevices.getUserMedia(constraints);
        video.srcObject = stream;
        await video.play();

        canvas.width = video.videoWidth;
        canvas.height = video.videoHeight;

        startBtn.disabled = true;
        stopBtn.disabled = false;
        scanLine.style.display = "block";
        downloadArea.style.display = "none";

        statusDiv.firstChild.textContent = "Scanning...";
        progressFill.style.width = "0%";

        requestAnimationFrame(scanLoop);
    } catch (err) {
        console.error("Camera error:", err);
        statusDiv.firstChild.textContent =
            "Error accessing camera: " + err.message;
    }
}

function stopCamera() {
    if (stream) {
        stream.getTracks().forEach((track) => track.stop());
        stream = null;
    }
    if (animationId) {
        cancelAnimationFrame(animationId);
        animationId = null;
    }
    video.srcObject = null;
    startBtn.disabled = false;
    stopBtn.disabled = true;
    scanLine.style.display = "none";
}

function scanLoop() {
    if (!stream || video.paused || video.ended) return;

    if (canvas.width !== video.videoWidth) {
        canvas.width = video.videoWidth;
        canvas.height = video.videoHeight;
    }

    ctx.drawImage(video, 0, 0, canvas.width, canvas.height);
    const imageData = ctx.getImageData(0, 0, canvas.width, canvas.height);

    const result = decoder.scan_frame(
        imageData.data,
        canvas.width,
        canvas.height,
    );
    const status = result.status;

    if (result.progress_total > 0) {
        const percent = (result.progress_current / result.progress_total) * 100;
        progressFill.style.width = `${percent}%`;
        statusDiv.firstChild.textContent = `Found ${result.progress_current} / ${result.progress_total} chunks...`;
    }

    if (status === ScanStatus.Complete) {
        stopCamera();
        statusDiv.firstChild.textContent = `Completed! Decoded: ${result.get_filename()}`;
        progressFill.style.width = "100%";
        enableDownload(result.get_filename(), result.get_file_data());
        return;
    }

    animationId = requestAnimationFrame(scanLoop);
}

function enableDownload(filename, data) {
    downloadArea.style.display = "block";
    downloadBtn.onclick = () => {
        const blob = new Blob([data], { type: "application/octet-stream" });
        const url = URL.createObjectURL(blob);
        const a = document.createElement("a");
        a.href = url;
        a.download = filename;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);
    };
}

run();
