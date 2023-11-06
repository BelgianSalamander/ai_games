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
    const pageTitleElement = document.getElementById('page-title');

    pageTitleElement.innerText = id + "'s Profile";

    fetch(`/api/profile?id=${id}`).then(response => response.json()).then(profile => {
        fetch("/api/lang").then(res => res.json()).then(langs => {
            lang_map = {};

            for (lang of langs) {
                lang_map[lang.id] = lang.name;
            }

            const username = profile.username;
            titleElement.innerText = "Profile - " + username;

            if (profile.privileged) {
                const nameSpan = document.createElement('span');
                nameSpan.innerText = username;
                nameSpan.style.color = 'green';

                const otherSpan = document.createElement('span');
                otherSpan.innerText = "'s Profile";

                pageTitleElement.innerHTML = '';
                pageTitleElement.appendChild(nameSpan);
                pageTitleElement.appendChild(otherSpan);

                document.getElementById('password-reset-container').style.display = 'block';

                let agent_list_container = document.getElementById("profile-agents-container");
                agent_list_container.style.display = 'block';
                let agent_list = document.getElementById("agent-list");

                for (agent of profile.agents) {
                    agent_list.appendChild(makeAgentElement(agent, lang_map));
                }
            } else {
                pageTitleElement.innerText = username + "'s Profile";
            }

            document.getElementById('username').innerText = username;
            document.getElementById('user-id').innerText = profile.id;
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