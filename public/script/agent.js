function onLoad() {
    //Get id from url
    const urlParams = new URLSearchParams(window.location.search);
    agent_id = urlParams.get('agent');

    const titleElement = document.getElementById('title');
    const pageHeadingElement = document.getElementById('page-heading');

    pageHeadingElement.innerText = "Agent " + agent_id

    fetch(`/api/agent?agent=${agent_id}&error=true&src=true`).then(response => response.json()).then(agent => {
        fetch("/api/lang", {"cache": "force-cache"}).then(res => res.json()).then(langs => {
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

            if ("owner" in agent) {
                const ownerLink = document.getElementById("agent-owner-link");
                ownerLink.href = "/pages/profile.html?id=" + agent.owner_id;
                if (agent.owner_id == getCookie("id")) {
                    ownerLink.innerText = "you!";
                } else {
                    ownerLink.innerText = agent.owner;
                }
            } else {
                document.getElementById("agent-owner").style.display = "none";
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