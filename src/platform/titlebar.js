// Static layout, installed once; Discord retains its real title and SVG icons.
(() => {
    if (window.top !== window) return;
    const install = () => {
        if (document.getElementById("discard-titlebar-layout")) return;
        const style = document.createElement("style");
        style.id = "discard-titlebar-layout";
        style.textContent = `
            #app-mount div[class^="base_"]:has(> div[class^="bar_"]) {
                --custom-app-top-bar-height: 32px;
                grid-template-rows: [top] 32px [titleBarEnd] min-content [noticeEnd] 1fr [contentEnd] min-content [end] !important;
            }
            #app-mount div[class^="base_"] > div[class^="bar_"] {
                height: 32px !important;
                min-height: 32px !important;
                box-sizing: border-box;
                padding-right: 140px !important;
                background: #121214 !important;
                app-region: drag;
                -webkit-app-region: drag;
            }
            #app-mount div[class^="base_"] > div[class^="bar_"] > div[class^="title_"] {
                position: absolute !important;
                left: 50% !important;
                right: auto !important;
                width: max-content !important;
                max-width: calc(100% - 460px);
                transform: translateX(-50%);
                overflow: hidden;
                white-space: nowrap;
                pointer-events: none;
                app-region: drag;
                -webkit-app-region: drag;
            }
            #app-mount div[class^="base_"] > div[class^="bar_"] > div[class^="trailing_"],
            #app-mount div[class^="base_"] > div[class^="bar_"] > div[class^="trailing_"] * {
                app-region: no-drag;
                -webkit-app-region: no-drag;
            }
        `;
        document.documentElement.appendChild(style);
    };
    if (document.readyState === "loading") {
        document.addEventListener("DOMContentLoaded", install, { once: true });
    } else {
        install();
    }
})();
