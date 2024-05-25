function onLoad() {
    fetch("/api/lang").then(d => d.json()).then(data => {
        let select = document.getElementById("agent-language");

        for (lang of data) {
            let option = document.createElement("option");
            option.innerText = lang.name;
            option.setAttribute("value", lang.id);

            select.appendChild(option);
        }
    });
}

function submit() {
    let name = document.getElementById("agent-name").value.trim();
    let language = document.getElementById("agent-language").value;
    let source = document.getElementById("source-code").value.trim();

    let feedback = document.getElementById("feedback");
    feedback.innerText = "";

    if (name.length === 0) {
        feedback.innerText = "Please enter a name!";
        return;
    }

    if (language.length === 0) {
        feedback.innerText = "Please select a language!";
        return;
    }

    if (source.length === 0) {
        feedback.innerText = "Please provide source code!";
    }

    console.log(name, language, source);

    let body = {
        "src": source,
        "lang": language,
        "name": name
    };

    fetch(`/api/add_agent?id=${getCookie("id")}`, {
        "method": "POST",
        "body": JSON.stringify(body)
    }).then(d => {
        if (d.status == 200) {
            d.json().then(data => {
                window.location = `/pages/agent.html?agent=${data['agent_id']}`;
            })
        } else {
            d.text().then(error => {
                feedback.innerText = error;
            })
        }
    });
}