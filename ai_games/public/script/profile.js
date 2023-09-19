let id = null;

function onLoad() {
    //Get id from url
    const urlParams = new URLSearchParams(window.location.search);
    id = urlParams.get('id');

    const titleElement = document.getElementById('title');
    const pageTitleElement = document.getElementById('page-title');

    pageTitleElement.innerText = id + "'s Profile";

    fetch(`/api/profile?id=${id}`).then(response => response.json()).then(profile => {
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
        } else {
            pageTitleElement.innerText = username + "'s Profile";
        }

        document.getElementById('username').innerText = username;
        document.getElementById('user-id').innerText = profile.id;
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