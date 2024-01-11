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

    endGame(element) {

    }
}

const GAME_MAP = {
    "Tic Tac Toe": new TicTacToe()
};

const MIN_DELAY = 250;
const eventQueue = [];
let lastUpdate = new Date();
let queueCallback = -1;

function processQueue() {
    if (queueCallback != -1) {
        clearTimeout(queueCallback);
    }
    queueCallback = -1;

    let currentTime = new Date();

    if (currentTime - lastUpdate >= MIN_DELAY) {
        if (eventQueue.length) {
            const e = document.getElementById("game-display");

            let packet = eventQueue[0];
            eventQueue.splice(0, 1);

            console.log("Processing packet", packet);
            if (packet.kind == "update") {
                gameEngine.updateGame(e, packet.data);
            } else if (packet.kind == "end") {
                gameEngine.endGame(e, packet.data);

                setTimeout(connect, 2000);
            } else {
                console.log("Invalid packet kind!", packet);
            }
        }

        lastUpdate = currentTime;

        queueCallback = setTimeout(processQueue, MIN_DELAY);
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
                    "kind": "update",
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