function onLoad() {
    //Get id from url
    const urlParams = new URLSearchParams(window.location.search);
    agent_id = urlParams.get('agent');

    const titleElement = document.getElementById('title');
    const pageTitleElement = document.getElementById('page-title');

    pageTitleElement.innerText = "Agent " + agent_id

    const main = document.getElementsByTagName("main")[0];

    fetch(`/api/agent?agent=${agent_id}&error=true`).then(response => response.json()).then(agent => {
        fetch("/api/lang").then(res => res.json()).then(langs => {
            lang_map = {};

            for (lang of langs) {
                lang_map[lang.id] = lang.name;
            }

            titleElement.innerText = "Agent - " + agent.name;
            pageTitleElement.innerText = "Agent - " + agent.name;

            if ("owner" in agent) {
                let owner = document.createElement("span");
                owner.innerText = "Owned by ";

                let owner_link = document.createElement("a")
                owner_link.href = "/public/profile.html?id=" + agent.owner_id;
                if (agent.owner_id == getCookie("id")) {
                    owner_link.innerText = "you!";
                } else {
                    owner_link.innerText = agent.owner;
                }
                owner.appendChild(owner_link);

                main.appendChild(owner);
                main.appendChild(document.createElement("br"));
            }

            let language = document.createElement("span");
            language.innerText = "Written in " + lang_map[agent.language];
            main.appendChild(language);

            main.appendChild(document.createElement("br"));

            let rating = document.createElement("span");
            rating.innerText = "Rating: " + Math.round(agent.rating);
            main.appendChild(rating);

            if (agent.removed) {
                main.appendChild(document.createElement("br"));

                let removed = document.createElement("span");
                removed.innerText = "Agent was removed";
                removed.style.color = "red";
                main.appendChild(removed);

                main.appendChild(document.createElement("br"));

                if ("error" in agent) {
                    let error = agent.error;

                    let pre = document.createElement("pre");
                    pre.style.backgroundColor = "#454b45";
                    let e = document.createElement("code");
                    e.innerText = error;
                    pre.appendChild(e);
                    main.appendChild(pre);
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