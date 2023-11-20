let id = null;

function makeAgentElement(agent, langs) {
    let element = document.createElement("div");
    element.classList.add("agent");

    let name = document.createElement("h3");
    name.innerText = agent.name;
    let as_link = document.createElement("a");
    as_link.href = "/public/agent.html?agent=" + agent.id;
    as_link.appendChild(name);
    element.appendChild(as_link);

    let language = document.createElement("span");
    language.innerText = "Written in " + langs[agent.language];
    language.style.color = "gray";
    element.appendChild(language);

    element.appendChild(document.createElement("br"));

    let rating = document.createElement("span");
    rating.innerText = "Rating: " + Math.round(agent.rating);
    element.appendChild(rating);

    if (agent.removed) {
        element.appendChild(document.createElement("br"));

        let removed = document.createElement("span");
        removed.innerText = "Agent was removed";
        removed.style.color = "red";
        element.appendChild(removed);
    }

    return element;
}

function onLoad() {
    //Get id from url
    const urlParams = new URLSearchParams(window.location.search);
    id = urlParams.get('id');

    const titleElement = document.getElementById('title');
    const pageTitleElement = document.getElementById('page-heading');

    fetch(`/api/profile?id=${id}`).then(response => response.json()).then(profile => {
        fetch("/api/lang").then(res => res.json()).then(langs => {
            lang_map = {};

            for (lang of langs) {
                lang_map[lang.id] = lang.name;
            }

            const username = profile.username;
            titleElement.innerText = "Profile - " + username;
            pageTitleElement.innerText = username + "'s Profile";

            if (profile.privileged) {
                const hidden = document.getElementById("hidden-info");
                hidden.style.display = "block";

                const agentGrid = document.getElementById("agent-grid");

                for (agent of profile.agents) {
                    const container = document.createElement("div");
                    container.classList.add("agent-container");

                    const nameLink = document.createElement("a");
                    nameLink.classList.add("agent-name-link");
                    nameLink.href = `/pages/agent.html?agent=${agent.id}`;

                    const nameElement = document.createElement("h3");
                    nameElement.classList.add("agent-name");
                    nameElement.innerText = agent.name;

                    nameLink.appendChild(nameElement);
                    container.appendChild(nameLink);

                    const languageElement = document.createElement("span");
                    languageElement.classList.add("agent-language");
                    if (agent.language in lang_map) {
                        languageElement.innerText = "Written in " + lang_map[agent.language];
                    } else {
                        languageElement.innerText = "Written in " + agent.language;
                    }
                    container.appendChild(languageElement);

                    const statusElement = document.createElement("span");
                    statusElement.classList.add("agent-status");
                    let displayStatus = false;

                    if (agent.partial && agent.removed) {
                        statusElement.classList.add("agent-status-compile-error");
                        statusElement.innerText = "Compile Error";
                        displayStatus = true;
                    } else if (agent.partial) {
                        statusElement.classList.add("agent-status-compiling");
                        statusElement.innerText = "Compiling...";
                        displayStatus = true;
                    } else if (agent.removed) {
                        statusElement.classList.add("agent-status-error");
                        statusElement.innerText = "Runtime Error";
                        displayStatus = true;
                    }

                    if (displayStatus) {
                        container.appendChild(document.createElement("br"));
                        container.appendChild(statusElement);
                    } else {
                        const rating = document.createElement("span");
                        rating.classList.add("agent-rating");
                        rating.innerText = "Rating: " + Math.round(agent.rating);
                        container.appendChild(document.createElement("br"));
                        container.appendChild(rating);

                        const gameCount = document.createElement("span");
                        gameCount.classList.add("agent-game-count");
                        gameCount.innerText = "Played " + agent.games_played + " games";
                        container.appendChild(document.createElement("br"));
                        container.appendChild(gameCount);
                    }

                    agentGrid.appendChild(container);
                }
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