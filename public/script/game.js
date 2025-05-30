let agents;
let gameEngine;
let activeSource;

COLOUR_CACHE = {};
NAME_CACHE = {};

function getColour(agentId) {
    if (agentId in COLOUR_CACHE) {
        return COLOUR_CACHE[agentId];
    } else {
        fetch(`/api/agent?agent=${agentId}&error=false&src=false`).then(res => res.json()).then(res => {
            COLOUR_CACHE[agentId] = res.colour;
            NAME_CACHE[agentId] = res.name;
        });

        return "#FF0000";
    }
}

class TicTacToe {
    makePlayerElement(id, extra) {
        const link = document.createElement("a");
        link.classList.add("playing-agent-name");
        link.href = `/pages/agent.html?agent=${id}`;
        link.style.color = getColour(id);

        if (!(id in NAME_CACHE)) {
            fetch(`/api/agent?agent=${id}`).then(res => res.json()).then(data => {
                link.innerText = data.name + extra;
                NAME_CACHE[id] = data.name;
                COLOUR_CACHE[id] = data.colour;
            });
        } else {
            link.innerText = NAME_CACHE[id] + extra;
        }

        return link;
    }

    startGame(element, players) {
        const globalContainer = document.createElement("div");
        globalContainer.style.display = "grid";
        globalContainer.style.gridTemplateColumns = "250px 1fr";

        const playerList = document.createElement("div");
        playerList.id = "tic-tac-toe-player-list";
        playerList.style.width = "250px";
        playerList.style.display = "flex";
        playerList.style.flexDirection = "column";
        playerList.style.justifyContent = "space-evenly";
        playerList.style.alignItems = "center";

        this.xColor = getColour(players[0]);
        this.oColor = getColour(players[1]);

        playerList.appendChild(this.makePlayerElement(players[0], " (X)"));
        const vs = document.createElement("span");
        vs.classList.add("tic-tac-toe-vs");
        vs.innerText = "Versus";
        playerList.appendChild(vs);
        playerList.appendChild(this.makePlayerElement(players[1], " (O)"));

        globalContainer.appendChild(playerList);

        const gridContainer = document.createElement("div");
        gridContainer.style.display = "grid";

        gridContainer.style.gridTemplateColumns = "200px 200px 200px";
        gridContainer.style.margin = "auto";
        gridContainer.style.justifyContent = "center";

        for (let r = 0; r < 3; r++) {
            for (let c = 0; c < 3; c++) {
                const cell = document.createElement("div");
                cell.id = "ttt-cell-" + r + "-" + c;
                cell.style.width = "200px";
                cell.style.height = "200px";
                cell.style.boxSizing = "border-box";
                cell.classList.add("tic-tac-toe-cell");

                const borderStyle = "2px solid black";

                if (c != 0) cell.style.borderLeft = borderStyle;
                if (c != 2) cell.style.borderRight = borderStyle;
                if (r != 0) cell.style.borderTop = borderStyle;
                if (r != 2) cell.style.borderBottom = borderStyle;

                gridContainer.appendChild(cell);
            }
        }

        globalContainer.appendChild(gridContainer);

        element.appendChild(globalContainer);
    }

    updateGame(element, data) {
        if (data.kind == "grid_state") {
            for (let r = 0; r < 3; r++) {
                for (let c = 0; c < 3; c++) {
                    const id = "ttt-cell-" + r + "-" + c;
                    const cell = document.getElementById(id);

                    if (data.data[r][c] == "Cross") {
                        cell.innerText = "X";
                        if (!cell.classList.contains("tic-tac-toe-cross")) {
                            cell.classList.add("tic-tac-toe-cross");
                        }
                        cell.style.color = this.xColor;
                    } else if (data.data[r][c] == "Nought") {
                        cell.innerText = "O";
                        if (!cell.classList.contains("tic-tac-toe-nought")) {
                            cell.classList.add("tic-tac-toe-nought");
                        }
                        cell.style.color = this.oColor;
                    }
                }
            }


            const lines = [
                [[0, 0], [0, 1], [0, 2]],
                [[1, 0], [1, 1], [1, 2]],
                [[2, 0], [2, 1], [2, 2]],

                [[0, 0], [1, 0], [2, 0]],
                [[0, 1], [1, 1], [2, 1]],
                [[0, 2], [1, 2], [2, 2]],

                [[0, 0], [1, 1], [2, 2]],
                [[2, 0], [1, 1], [0, 2]]
            ]

            for (let line of lines) {
                if (data.data[line[0][0]][line[0][1]] != "Empty" && data.data[line[0][0]][line[0][1]] == data.data[line[1][0]][line[1][1]] && data.data[line[0][0]][line[0][1]] == data.data[line[2][0]][line[2][1]]) {
                    console.log("Found win!");
                    for (let r = 0; r < 3; r++) {
                        for (let c = 0; c < 3; c++) {
                            const id = "ttt-cell-" + r + "-" + c;
                            const cell = document.getElementById(id);

                            if ((line[0][0] != r || line[0][1] != c) && (line[1][0] != r || line[1][1] != c) && (line[2][0] != r || line[2][1] != c)) {
                                cell.style.color = "#777";
                            }
                        }
                    }

                    break;
                }
            }
        }
    }

    shouldWaitForUpdate(data) {
        return true;
    }

    endGame(element) {

    }
}

const SNAKE_EMPTY = "#4d4d4d";
const SNAKE_FOOD = "#f5f11b";

class NZOISnake {
    makePlayerElement(id, extra) {
        const link = document.createElement("a");
        link.classList.add("playing-agent-name");
        link.href = `/pages/agent.html?agent=${id}`;
        link.style.color = getColour(id);

        if (!(id in NAME_CACHE)) {
            fetch(`/api/agent?agent=${id}`).then(res => res.json()).then(data => {
                link.innerText = data.name + extra;
                NAME_CACHE[id] = data.name;
                COLOUR_CACHE[id] = data.colour;
            });
        } else {
            link.innerText = NAME_CACHE[id] + extra;
        }

        return link;
    }

    startGame(element, players) {
        const globalContainer = document.createElement("div");
        globalContainer.style.display = "grid";
        globalContainer.style.gridTemplateColumns = "350px 1fr";

        const playerList = document.createElement("div");
        playerList.id = "snake-player-list";
        playerList.style.width = "350px";
        playerList.style.justifyContent = "space-evenly";
        playerList.style.alignItems = "center";

        this.colours = [];
        this.scoreElements = [];
        this.scores = [];

        for (const player of players) {
            let colour = getColour(player);
            this.colours.push(colour);

            const link = document.createElement("a");
            link.classList.add("playing-agent-name");

            link.href = `/pages/agent.html?agent=${player}`;
            link.style.color = colour;

            if (!(player in NAME_CACHE)) {
                fetch(`/api/agent?agent=${player}`).then(res => res.json()).then(data => {
                    link.innerText = data.name;
                    NAME_CACHE[player] = data.name;
                    COLOUR_CACHE[player] = data.colour;
                });
            } else {
                link.innerText = NAME_CACHE[player];
            }

            playerList.appendChild(link);

            let scoreElement = document.createElement("span");
            scoreElement.classList.add("snake-score");
            scoreElement.innerText = "0";
            this.scores.push(0);

            this.scoreElements.push(scoreElement);
            playerList.appendChild(scoreElement);
        }

        globalContainer.appendChild(playerList);

        const gridContainer = document.createElement("div");
        gridContainer.style.display = "grid";
        gridContainer.id = "grid-container";

        gridContainer.style.margin = "auto";
        gridContainer.style.justifyContent = "center";

        globalContainer.appendChild(gridContainer);

        element.appendChild(globalContainer);
    }

    updateGame(element, data) {
        let packetKind = data[0];
        let packetData = data[1];
        if (packetKind == "dimensions") {
            this.rows = packetData[0];
            this.cols = packetData[1];

            console.log(`Grid is ${this.rows} by ${this.cols}`);

            let gridContainer = document.getElementById("grid-container");

            gridContainer.style.gridTemplateColumns = "1fr ".repeat(this.cols);
            gridContainer.style.gridTemplateRows = "1fr ".repeat(this.rows);

            let size = 500 / Math.max(this.rows, this.cols);
            size = `${size}px`;

            for (let r = 0; r < this.rows; r++) {
                for (let c = 0; c < this.cols; c++) {
                    const cell = document.createElement("div");
                    cell.id = "snake-cell-" + r + "-" + c;
                    cell.classList.add("snake-cell");

                    cell.style.width = size;
                    cell.style.height = size;

                    cell.style.backgroundColor = SNAKE_EMPTY;

                    gridContainer.appendChild(cell);
                }
            }
        } else if (packetKind == "grid") {
            for (let r = 0; r < this.rows; r++) {
                for (let c = 0; c < this.cols; c++) {
                    const cell = document.getElementById("snake-cell-" + r + "-" + c);

                    let val = packetData[r][c];

                    if (val == -1) cell.style.backgroundColor = SNAKE_FOOD;
                    else if (val == 0) cell.style.backgroundColor = SNAKE_EMPTY;
                    else {
                        let player_index = val - 1;
                        cell.style.backgroundColor = this.colours[player_index];
                    }
                }
            }
        } else if (packetKind == "upd") {
            for (let arr of packetData) {
                const newVal = arr[0];

                for (let i = 1; i < arr.length; i += 2) {
                    const row = arr[i];
                    const col = arr[i+1];

                    const cell = document.getElementById("snake-cell-" + row + "-" + col);

                    let colour;
                    if (newVal == -1) colour = SNAKE_FOOD;
                    else if (newVal == 0) colour = SNAKE_EMPTY;
                    else colour = this.colours[newVal - 1];

                    cell.style.backgroundColor = colour;
                }
            }
        } else if (packetKind == "scr") {
            for (let i = 0; i < this.scoreElements.length; i++) {
                //this.scoreElements[i].innerText = `${packetData[i]}`;
                this.scores[i] += packetData[i];
                this.scoreElements[i].innerText = `${this.scores[i]}`
            }
        }
    }

    shouldWaitForUpdate(data) {
        return data[0] == "grid" || data[0] == "upd";
    }

    endGame(element) {

    }
}

const GAME_MAP = {
    "Tic Tac Toe": new TicTacToe(),
    "Snake": new NZOISnake()
};

let MIN_DELAY = 20;
const eventQueue = [];
let lastUpdate = new Date();
let queueCallback = -1;

function processQueue() {
    if (queueCallback != -1) {
        clearTimeout(queueCallback);
    }
    queueCallback = -1;

    if (!eventQueue.length) return;

    let currentTime = new Date();

    let mustWait = true;
    if (eventQueue[0].kind == "upd") {
        mustWait = gameEngine.shouldWaitForUpdate(eventQueue[0].data);
    }
    if (!mustWait || currentTime - lastUpdate >= MIN_DELAY) {
        const e = document.getElementById("game-display");

        let packet = eventQueue[0];
        eventQueue.splice(0, 1);

        console.log("Processing packet", packet);
        if (packet.kind == "upd") {
            gameEngine.updateGame(e, packet.data);
        } else if (packet.kind == "end") {
            gameEngine.endGame(e, packet.data);

            setTimeout(connect, 3000);
        } else {
            console.log("Invalid packet kind!", packet);
        }

        lastUpdate = currentTime;

        queueCallback = setTimeout(processQueue, 0);
    } else {
        let needed = Math.max(0, MIN_DELAY - (currentTime - lastUpdate));

        queueCallback = setTimeout(processQueue, needed);
    }
}

function onLoad() {
    fetch("/api/agent_leaderboard").then(x => x.json()).then(data => {
        agents = data;
        agents.sort((a, b) => {
            if (a.name < b.name) {
                return -1;
            } else if (a.name > b.name) {
                return 1;
            } else {
                return 0;
            }
        });

        for (agent of data) {
            COLOUR_CACHE[agent.id] = agent.colour;
            NAME_CACHE[agent.id] = agent.name;
        }
    });

    /*ws = new WebSocket("ws://172.31.180.162:42070/");
    ws.onmessage = */
}

function updateAgentSuggestions(e) {
    let suggestionsElement = document.getElementById("agent-name-suggestions");

    let content = e.value;

    suggestionsElement.style.display = "block";

    let suggestions = [];

    for (agent of agents) {
        if (agent.name.toLowerCase().startsWith(content.toLowerCase())) {
            suggestions.push(agent.name);

            if (suggestions.length >= 5) break;
        }
    }

    console.log(suggestions);

    suggestionsElement.innerHTML = "";

    for (const suggestion of suggestions) {
        const span = document.createElement("span");
        span.innerText = suggestion;
        span.onclick = () => {
            e.value = suggestion;
            console.log("Clicked!");
            document.getElementById("agent-name-suggestions").style.display = "none";
        }

        suggestionsElement.appendChild(span);
    }
}

function hideSuggestions() {
    setTimeout(
        () => document.getElementById("agent-name-suggestions").style.display = "none",
        1000
    );
    console.log("Hide");
}

function connect() {
    const agentName = document.getElementById("agent-name").value;
    let agentId = -1;

    for (agent of agents) {
        if (agentName == agent.name) {
            agentId = agent.id;
            break;
        }
    }

    let req;

    if (agentId == -1) {
        req = "\"Any\"";
    } else {
        req = JSON.stringify({ "WithPlayer": agentId });
    }

    let url = `/bruh?req=${encodeURIComponent(req)}`;

    if (activeSource) {
        activeSource.close();
    }

    activeSource = new EventSource(url);
    activeSource.onmessage = (m) => {
        json = JSON.parse(m.data);
        console.log(json);
        const e = document.getElementById("game-display");

        if (json.kind == "connect") {
            e.innerHTML = "";
            gameEngine = GAME_MAP[json.data.kind]
            gameEngine.startGame(e, json.data.players);
            lastUpdate = new Date();

            eventQueue.length = 0;

            for (p of json.data.history) {
                eventQueue.push({
                    "kind": "upd",
                    "data": JSON.parse(p)
                });
            }

            processQueue();
        } else {
            eventQueue.push(json);

            if (json.kind == "end") {
                activeSource.close();
                activeSource = undefined;
            }

            processQueue();
        }
    };
}