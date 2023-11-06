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
    let name = document.getElementById("agent-name").value;
    let language = document.getElementById("agent-language").value;
    let source = document.getElementById("agent-source").value;

    if (name.length === 0 || source.length === 0) return;

    console.log(name, language, source);

    let body = {
        "src": source,
        "lang": language,
        "name": name
    };

    let feedback = document.getElementById("feedback");
    feedback.innerText = "";

    fetch(`/api/add_agent?id=${getCookie("id")}`, {
        "method": "POST",
        "body": JSON.stringify(body)
    }).then(d => {
        if (d.status == 200) {
            d.json().then(data => {

            })
        } else {
            d.text().then(error => {
                feedback.innerText = error;
            })
        }
    });
}