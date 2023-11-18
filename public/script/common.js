function getCookie(name) {
    const value = `; ${document.cookie}`;
    const parts = value.split(`; ${name}=`);
    if (parts.length === 2) return parts.pop().split(';').shift();
}

function areObjectsEqual(o1, o2) {
    if (o1 === o2) return true;

    if (typeof o1 != typeof o2) return false;

    if (typeof o1 === "object") {
        keys = Object.keys(o1);

        if (Object.keys(o2).length != keys.length) {
            return false;
        }

        for (key of keys) {
            if (!areObjectsEqual(o1[key], o2[key])) {
                return false;
            }
        }

        return true;
    } else {
        return false;
    }
}

function logOut() {
    console.log(document.cookie);
    document.cookie = "id=;path=/;expires=Thu, 01 Jan 1970 00:00:01 GMT"
    document.cookie = "password=;path=/;expires=Thu, 01 Jan 1970 00:00:01 GMT"

    location.reload();
}

function makeLoggedOut(menu) {
    let login = document.createElement("a");
    login.innerText = "Log In";
    login.href = "/public/login.html";

    menu.appendChild(login);
}

function makeLoggedIn(menu, id) {
    fetch(`/api/profile?id=${id}`).then(res => res.json()).then(data => {
        const username = document.createElement("a");
        username.innerText = data.username;
        username.href = `/public/profile.html?id=${id}`

        username.style.textDecoration = "none";
        username.style.fontWeight = "bold";
        username.style.fontSize = "20pt";

        menu.appendChild(username);

        const logout = document.createElement("div");
        logout.style.display = "flex";
        logout.style.flexDirection = "row";
        logout.style.alignContent = "center";

        const icon = document.createElement("img");
        icon.src = "/public/assets/logout.png";
        icon.style.height = "20px";
        logout.appendChild(icon);
        const text = document.createElement("span");
        text.innerText = "Log Out";
        logout.appendChild(text);

        logout.onclick = (ev) => {
            if (ev.button == 0) {
                logOut();
            }
        }
        
        menu.appendChild(logout);
    });
}

function commonLoad() {
    let profileOptions = document.getElementById("header-profile-options");

    id = getCookie("id");
    password = getCookie("password");
    if (!id || !password) {
        makeLoggedOut(profileOptions);
    } else {
        fetch(`/api/auth?id=${id}&password=${password}`).then(res => {
            if (res.status != 200) {
                logOut();
            } else {
                return res.json();
            }
        }).then(data => {
            if (!data.correct) logOut();

            makeLoggedIn(profileOptions, id);
        });
    }
}