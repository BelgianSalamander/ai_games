function login(username, password) {
    const expiry = new Date();
    expiry.setDate(expiry.getTime() + 24 * 60 * 60 * 1000); // 24 hours

    const extra = `;path=/;SameSite=Strict;expires=${expiry.toUTCString()}`;

    fetch(`/api/profile?username=${username}`).then(res => res.json()).then(data => {
        document.cookie = `id=${data.id}` + extra;
        document.cookie = `password=${password}` + extra;

        window.location = `/public/profile.html?id=${data.id}`;
    });
}

function tryLogin() {
    const username = document.getElementById("username").value;
    const password = document.getElementById("password").value;

    const result = document.getElementById("result");

    fetch(`/api/auth?username=${username}&password=${password}`).then(res => {
        if (res.status == 404) {
            result.style.color = 'red';
            result.innerText = "Incorrect Username";
        } else if (res.status != 200) {
            result.style.color = 'red';
            result.innerText = "Error";
        } else {
            res.json().then(data => {
                if (!data.correct) {
                    result.style.color = 'red';
                    result.innerText = "Incorrect Password";
                } else {
                    result.style.color = 'green';
                    result.innerText = "Correct";
                    login(username, password);
                }
            })
        }
    })
}