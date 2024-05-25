// From https://stackoverflow.com/questions/2353211/hsl-to-rgb-color-conversion
/**
 * Converts an HSL color value to RGB. Conversion formula
 * adapted from https://en.wikipedia.org/wiki/HSL_color_space.
 * Assumes h, s, and l are contained in the set [0, 1] and
 * returns r, g, and b in the set [0, 255].
 *
 * @param   {number}  h       The hue
 * @param   {number}  s       The saturation
 * @param   {number}  l       The lightness
 * @return  {Array}           The RGB representation
 */
function hslToRgb(h, s, l) {
    let r, g, b;

    if (s === 0) {
        r = g = b = l; // achromatic
    } else {
        const q = l < 0.5 ? l * (1 + s) : l + s - l * s;
        const p = 2 * l - q;
        r = hueToRgb(p, q, h + 1 / 3);
        g = hueToRgb(p, q, h);
        b = hueToRgb(p, q, h - 1 / 3);
    }

    return [Math.round(r * 255), Math.round(g * 255), Math.round(b * 255)];
}

function hueToRgb(p, q, t) {
    if (t < 0) t += 1;
    if (t > 1) t -= 1;
    if (t < 1 / 6) return p + (q - p) * 6 * t;
    if (t < 1 / 2) return q;
    if (t < 2 / 3) return p + (q - p) * (2 / 3 - t) * 6;
    return p;
}

/**
 * Converts an RGB color value to HSL. Conversion formula
 * adapted from http://en.wikipedia.org/wiki/HSL_color_space.
 * Assumes r, g, and b are contained in the set [0, 255] and
 * returns h, s, and l in the set [0, 1].
 *
 * @param   {number}  r       The red color value
 * @param   {number}  g       The green color value
 * @param   {number}  b       The blue color value
 * @return  {Array}           The HSL representation
 */
function rgbToHsl(r, g, b) {
    (r /= 255), (g /= 255), (b /= 255);
    const vmax = Math.max(r, g, b), vmin = Math.min(r, g, b);
    let h, s, l = (vmax + vmin) / 2;

    if (vmax === vmin) {
        return [0, 0, l]; // achromatic
    }

    const d = vmax - vmin;
    s = l > 0.5 ? d / (2 - vmax - vmin) : d / (vmax + vmin);
    if (vmax === r) h = (g - b) / d + (g < b ? 6 : 0);
    if (vmax === g) h = (b - r) / d + 2;
    if (vmax === b) h = (r - g) / d + 4;
    h /= 6;

    return [h, s, l];
}

// From https://stackoverflow.com/questions/5623838/rgb-to-hex-and-hex-to-rgb
function hexToRgb(hex) {
    var result = /^#?([a-fA-F\d]{2})([a-fA-F\d]{2})([a-fA-F\d]{2})$/i.exec(hex);
    return result ? [parseInt(result[1], 16), parseInt(result[2], 16), parseInt(result[3], 16)] : null;
}

function setCanvasHue(hue) {
    const canvas = document.getElementById("colour-picker-canvas");
    const ctx = canvas.getContext("2d");


    for (let i = 0; i < 250; i++) {
        for (let j = 0; j < 120; j++) {
            let s = i / 249 * 0.8 + 0.2;
            let l = j / 119 * 0.6 + 0.3;

            ctx.fillStyle = `hsl(${hue}, ${s * 100}%, ${l * 100}%)`;
            ctx.fillRect(i, 119 - j, 1, 1);
        }
    }
}

function updateColor(h, s, l, update) {
    const canvasSelectorCircle = document.getElementById("sl-slider-circle");

    canvasSelectorCircle.style.backgroundColor = `hsl(${h}, ${s * 100}%, ${l * 100}%)`;

    document.getElementById("agent-colour-block").style.backgroundColor = `hsl(${h}, ${s * 100}%, ${l * 100}%)`;

    const slider = document.getElementById("colour-slider");
    const sliderCircle = document.getElementById("colour-slider-circle");
    const canvasSelector = document.getElementById("sl-slider");

    slider.style.left = `${h / 360 * 249}px`;
    sliderCircle.style.backgroundColor = `hsl(${h}, 100%, 50%)`;

    canvasSelector.style.left = `${(s - 0.2) / 0.8 * 249}px`;
    canvasSelector.style.top = `${119 - (l - 0.3) / 0.6 * 119}px`;

    if (update) {
        callUpdate(() => {
            let rgb = hslToRgb(h / 360, s, l);

            fetch(`/api/set_colour?id=${id}&agent=${agent_id}&r=${rgb[0]}&g=${rgb[1]}&b=${rgb[2]}`, {
                "method": "POST"
            });
        })
    }
}

function setupColourPicker(baseColor) {

    let baseRgb = hexToRgb(baseColor);
    if (baseRgb == null) {
        baseRgb = [255, 0, 0];
    }

    let baseHsl = rgbToHsl(baseRgb[0], baseRgb[1], baseRgb[2]);

    const slider = document.getElementById("colour-slider");
    const sliderCircle = document.getElementById("colour-slider-circle");
    const hueLine = document.getElementById("hue-line");

    let draggingHue = false;

    let position = 0;
    let falsePosition = 0;

    let hue = baseHsl[0] * 360;
    let saturation = baseHsl[1];
    let lightness = baseHsl[2];

    let colorChanged = false;

    updateColor(hue, saturation, lightness, false);
    setCanvasHue(hue);

    slider.onmousedown = e => {
        console.log("clicked");
        draggingHue = true;

        e.stopPropagation();
    };

    document.onmousemove = e => {
        if (draggingHue) {
            falsePosition += e.movementX;
            position = Math.max(0, Math.min(250, falsePosition));

            hue = position / 250 * 360;
            setCanvasHue(hue);

            updateColor(hue, saturation, lightness, true);
        }
    }

    document.onmouseup = e => {
        draggingHue = false;

        falsePosition = position;
    }

    hueLine.onclick = e => {
        position = e.layerX;
        falsePosition = position;

        hue = position / 250 * 360;
        setCanvasHue(hue);

        updateColor(hue, saturation, lightness, true);
    }

    const canvas = document.getElementById("colour-picker-canvas");
    const canvasSelector = document.getElementById("sl-slider");

    canvas.onclick = e => {
        let x = e.layerX / 249;
        let y = (119 - e.layerY) / 119;

        saturation = x * 0.8 + 0.2;
        lightness = y * 0.6 + 0.3;

        updateColor(hue, saturation, lightness, true);
    }

    document.getElementById("colour-container").onclick = e => {
        let el = document.getElementById("colour-picker");
        el.style.display = "block";

        e.stopPropagation();
    }

    document.body.onclick = e => {
        let el = document.getElementById("colour-picker");

        if (el.style.display == "block") {

        }

        el.style.display = "none";
    }
}

let updateHandle = -1;
function callUpdate(f) {
    if (updateHandle != -1) {
        clearTimeout(updateHandle);
        updateHandle = -1;
    }

    updateHandle = setTimeout(f, 1000);
}

function onLoad() {
    //Get id from url
    const urlParams = new URLSearchParams(window.location.search);
    agent_id = urlParams.get('agent');

    const titleElement = document.getElementById('title');
    const pageHeadingElement = document.getElementById('page-heading');

    pageHeadingElement.innerText = "Agent " + agent_id

    fetch(`/api/agent?agent=${agent_id}&error=true&src=true`).then(response => response.json()).then(agent => {
        fetch("/api/lang", { "cache": "force-cache" }).then(res => res.json()).then(langs => {
            lang_map = {};

            for (lang of langs) {
                lang_map[lang.id] = lang.name;
            }

            titleElement.innerText = "Agent - " + agent.name;
            pageHeadingElement.innerText = "Agent - " + agent.name;

            document.getElementById("agent-name").innerText = "Name: '" + agent.name + "'";

            let language = agent.language;
            if (language in lang_map) {
                language = lang_map[language];
            }
            document.getElementById("agent-language").innerText = "Written in " + language;

            let authed = false;

            if ("owner" in agent) {
                const ownerLink = document.getElementById("agent-owner-link");
                ownerLink.href = "/pages/profile.html?id=" + agent.owner_id;
                if (agent.owner_id == getCookie("id")) {
                    ownerLink.innerText = "you!";
                    authed = true;
                } else {
                    ownerLink.innerText = agent.owner;
                }
            } else {
                document.getElementById("agent-owner").style.display = "none";
            }

            let colourElement = document.getElementById("agent-colour-block");
            colourElement.style.backgroundColor = agent.colour;

            if (authed) {
                colourElement.setAttribute("auth", "1");

                setupColourPicker(agent.colour);
            }

            document.getElementById("agent-rating").innerText = "Rating: " + Math.round(agent.rating);
            document.getElementById("agent-games-played").innerText = "Num Games Played: " + agent.games_played;

            let status, statusClass;
            if (agent.removed && agent.partial) {
                status = "Compile Error";
                statusClass = "agent-status-compile-error";
            } else if (agent.removed) {
                status = "Runtime Error";
                statusClass = "agent-status-error";
            } else if (agent.partial) {
                status = "Compiling...";
                statusClass = "agent-status-compiling";
            } else {
                status = "Ok!";
                statusClass = "agent-status-alg";
            }

            const statusElement = document.getElementById("agent-status");
            statusElement.innerText = status;
            statusElement.classList.add(statusClass);

            if ("error" in agent) {
                document.getElementById("agent-error").style.display = "block";
                document.getElementById("agent-error-display").innerText = agent.error;
            }

            if ("src" in agent) {
                document.getElementById("agent-source").style.display = "block";
                document.getElementById("agent-source-display").innerText = agent.src;
            }
        });
    });
}

function resetPassword() {
    fetch(`/api/reset_password?id=${id}`, {
        method: 'POST'
    }).then(response => response.text()).then(text => {
        document.getElementById('new-password').innerText = text;

        if (getCookie("id") == id) {
            date = new Date();
            date.setTime(date.getTime() + (6 * 60 * 60 * 1000));

            document.cookie = `password=${text};expires=${date.toUTCString()};path=/;SameSite=Strict`;
        }
    });
}