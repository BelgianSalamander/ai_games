let authed = false;

function getCookie(name) {
    const value = `; ${document.cookie}`;
    const parts = value.split(`; ${name}=`);
    if (parts.length === 2) return parts.pop().split(';').shift();
}

function setPassword() {
    const element = document.getElementById('admin-password');
    const content = element.value;

    const expiry = new Date();
    expiry.setDate(expiry.getTime() + 6 * 60 * 60 * 1000); // 6 hours
    document.cookie = `admin=${content};expires=${expiry.toUTCString()};path=/;SameSite=Strict`;
    authed = false;

    verifyPassword();
}

function onLoad() {
    setInterval(updateProfileList, 1000);
    updatePassword();
}

function verifyPassword() {
    const element = document.getElementById('admin-password-status');
    fetch('/admin/verify').then(response => {
        if (response.status === 200) {
            element.style.color = '#555';
            element.innerText = 'Password is correct';
            authed = true;
            updateProfileList(true);
        } else {
            element.style.color = 'red';
            element.innerText = 'Password is incorrect';
            authed = false;
        }
    });
}

function updatePassword() {
    const element = document.getElementById('admin-password');

    const password = getCookie('admin');

    if (password) {
        element.value = password;
        verifyPassword();
    }
}

function fullReset() {
    if (confirm("Are you sure you want to reset everything? This action is irreversible!")) {
        fetch(`/admin/full_reset`, {
            method: 'POST'
        });
    }
}

function generateProfileTable(data) {
    table = document.getElementById('profile-list');
    table.innerHTML = '';

    const HEADERS = ['Id', 'Username', 'Password', 'No. Agents', 'Controls'];

    const headerRow = document.createElement('tr');
    for (const header of HEADERS) {
        const th = document.createElement('th');
        th.innerText = header;
        headerRow.appendChild(th);
    }
    table.appendChild(headerRow);

    for (const profile of data) {
        const row = document.createElement('tr');

        const id = document.createElement('td');
        id.innerText = profile.id;
        row.appendChild(id);

        const username = document.createElement('td');
        username.style.alignContent = "left";
        const link = document.createElement('a');
        link.href = `/pages/profile.html?id=${profile.id}`;
        link.innerText = profile.username;

        link.style.color = "var(--dark-text)";
        link.style.textDecoration = "none";

        username.appendChild(link);
        row.appendChild(username);

        const passwordContainer = document.createElement('td');
        const password = document.createElement('span');
        password.innerText = profile.password;
        password.classList.add('show-on-hover');
        passwordContainer.appendChild(password);
        row.appendChild(passwordContainer);

        const agentsContainer = document.createElement('td');
        const agents = document.createElement('input');
        agents.setAttribute("type", "number");
        agents.setAttribute("min", "0");
        agents.value = profile.num_agents_allowed;
        agents.onchange = e => {
            console.log(e.target.value);
            fetch(`/admin/set_profile_agents?id=${profile.id}&agents=${e.target.value}`, {
                method: 'POST'
            });
            updateProfileList(false);
        };
        row.appendChild(agents);

        const controls = document.createElement('td');
        const delButton = document.createElement('button');
        delButton.innerText = 'Delete';
        delButton.onclick = () => {
            fetch(`/admin/delete_profile?id=${profile.id}`, {
                method: 'POST'
            }).then(response => {
                if (response.status === 200) {
                    console.log('Profile deleted!');
                    updateProfileList(false);
                } else {
                    console.log('Profile deletion failed!');
                    console.log(response);
                    response.text().then(text => console.log(text));
                }
            });
        };
        controls.appendChild(delButton);

        const resetButton = document.createElement('button');
        resetButton.innerText = 'Reset Password';
        resetButton.onclick = () => {
            resetPassword(profile.id);
        };
        controls.appendChild(resetButton);
        row.appendChild(controls);

        table.appendChild(row);
    }
}

var prevData = [];

function updateProfileList(force) {
    if (!authed) return;
    fetch('/admin/profiles').then(response => {
        if (response.status === 200) {
            return response.json();
        } else {
            verifyPassword();
        }
    }).then(data => {
        if (force || !areObjectsEqual(data, prevData)) {
            generateProfileTable(data);
            prevData = data;
        }
    });
}

function makeNewProfile(username, numAgents) {
    if (!authed) return;

    if (username.length == 0) {
        const element = document.getElementById('new-profile-status');
        element.style.color = "red";
        element.innerText = "Provide a username";
        return;
    }

    fetch(`/admin/new_profile?username=${username}&agents=${numAgents}`, {
        method: 'POST'
    }).then(response => {
        const element = document.getElementById('new-profile-status');
        if (response.status === 200) {
            console.log('Profile created!');

            element.style.color = 'green';
            element.innerText = 'Profile created!';
        } else {
            console.log('Profile creation failed!');
            console.log(response);

            element.style.color = 'red';
            response.text().then(text => {
                element.innerText = text;
            });
        }
    });
}

function addProfile() {
    if (!authed) return;

    const username = document.getElementById('new-profile-username').value;
    let numAgents = 0;
    const agentElement = document.getElementById('new-profile-agents');
    if (agentElement.value != "") {
        numAgents = agentElement.value;
    }

    makeNewProfile(username, numAgents);
}

function resetPassword(id) {
    fetch(`/api/reset_password?id=${id}`, {
        method: 'POST'
    }).then(response => response.text()).then(text => {
        if (getCookie("id") == id) {
            date = new Date();
            date.setTime(date.getTime() + (6 * 60 * 60 * 1000));

            document.cookie = `password=${text};expires=${date.toUTCString()};path=/;SameSite=Strict`;
        }

        updateProfileList();
    });
}