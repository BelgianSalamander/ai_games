function login(username, password) {
    const expiry = new Date();
    expiry.setDate(expiry.getTime() + 24 * 60 * 60 * 1000); // 24 hours

    const extra = `;path=/;SameSite=Strict;expires=${expiry.toUTCString()}`;

    fetch(`/api/profile?username=${username}`).then(res => res.json()).then(data => {
        document.cookie = `id=${data.id}` + extra;
        document.cookie = `password=${password}` + extra;

        window.location = `/pages/profile.html?id=${data.id}`;
    });
}

function tryLogin() {
    console.log("Trying to log in!");
    const username = document.getElementById("username-input").value;
    const password = document.getElementById("password-input").value;

    const feedback = document.getElementById("feedback");

    fetch(`/api/auth?username=${username}&password=${password}`).then(res => {
        if (res.status == 404) {
            feedback.style.color = 'red';
            feedback.innerText = "Incorrect Username";
        } else if (res.status != 200) {
            feedback.style.color = 'red';
            feedback.innerText = "Error";
        } else {
            res.json().then(data => {
                if (!data.correct) {
                    feedback.style.color = 'red';
                    feedback.innerText = "Incorrect Password";
                } else {
                    feedback.style.color = 'green';
                    feedback.innerText = "Correct";
                    login(username, password);
                }
            })
        }
    })
}