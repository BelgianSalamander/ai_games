function getCookie(name) {
    const value = `; ${document.cookie}`;
    const parts = value.split(`; ${name}=`);
    if (parts.length === 2) return parts.pop().split(';').shift();
}

function makeLogin(logStatus) {
    const username = document.createElement("a");
    username.innerText = "Log In";
    username.href = `/public/login.html`

    username.style.textDecoration = "none";
    username.style.color = "fee";
    username.style.display = "inline-block";
    username.style.fontWeight = "bold";
    username.style.fontSize = "20pt";

    logStatus.appendChild(username);
}

function makeProfile(logStatus, id, password) {
    fetch(`/api/profile?id=${id}`).then(res => res.json()).then(data => {
        const username = document.createElement("a");
        username.innerText = data.username;
        username.href = `/public/profile.html?id=${id}`

        username.style.textDecoration = "none";
        username.style.color = "#fff";
        username.style.display = "inline-block";
        username.style.fontWeight = "bold";
        username.style.fontSize = "20pt";

        logStatus.appendChild(username);
    });
}

function logOut() {
    console.log(document.cookie);
    document.cookie = "id=;path=/;expires=Thu, 01 Jan 1970 00:00:01 GMT"
    document.cookie = "password=;path=/;expires=Thu, 01 Jan 1970 00:00:01 GMT"

    location.reload();
}

function commonLoad() {
    let header = document.getElementsByTagName("header");

    if (header.length != 1) {
        console.log("ERROR: Found multiple headers!!");
        return;
    } else {
        header = header[0];
    }

    const logStatusContainer = document.createElement("div");
    logStatusContainer.id = "log-status-container";
    header.appendChild(logStatusContainer);

    const logStatus = document.createElement("div");
    logStatus.style.margin = "10px";
    logStatusContainer.appendChild(logStatus);

    id = getCookie("id");
    password = getCookie("password");
    if (!id || !password) {
        makeLogin(logStatus);
    } else {
        fetch(`/api/auth?id=${id}&password=${password}`).then(res => {
            if (res.status != 200) {
                logOut();
            } else {
                return res.json();
            }
        }).then(data => {
            if (!data.correct) logOut();

            makeProfile(logStatus, id, password);
        });
    }
}