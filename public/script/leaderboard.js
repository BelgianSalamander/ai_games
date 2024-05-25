function onLoad() {
    const table = document.getElementById("leaderboard");

    const headings = ["Rank", "Name", "Rating", "Owner", "Games Played"];
    const headingRow = document.createElement("tr");

    for (heading of headings) {
        const e = document.createElement("th");
        e.innerText = heading;
        headingRow.appendChild(e);
    }

    table.appendChild(headingRow);

    fetch("/api/agent_leaderboard").then(r => r.json()).then(data => {
        for (let i = 0; i < data.length; i++) {
            const agent = data[i];
            const row = document.createElement("tr");

            const rankElement = document.createElement("td");
            rankElement.classList.add("agent-rank");
            rankElement.innerText = i + 1;
            row.appendChild(rankElement);

            const nameElement = document.createElement("td");
            rankElement.classList.add("agent-name");
            const nameLinkElement = document.createElement("a");
            nameLinkElement.classList.add("agent-name-link");
            nameLinkElement.href = `/pages/agent.html?agent=${agent.id}`;
            nameLinkElement.innerText = agent.name;
            nameLinkElement.style.color = agent.colour;
            nameElement.appendChild(nameLinkElement);
            row.appendChild(nameElement);

            const ratingElement = document.createElement("td");
            ratingElement.classList.add("agent-rating");
            ratingElement.innerText = agent.rating;
            row.appendChild(ratingElement);

            const ownerElement = document.createElement("td");
            ownerElement.classList.add("agent-owner");
            if ("owner" in agent) {
                const ownerLink = document.createElement("a");
                ownerLink.classList.add("agent-owner-link");
                ownerLink.href = `/pages/profile.html?id=${agent.owner_id}`;
                ownerLink.innerText = agent.owner;
                ownerElement.appendChild(ownerLink);
            }
            row.appendChild(ownerElement);

            const gamePlayedElement = document.createElement("td");
            gamePlayedElement.classList.add("agent-game-played");
            gamePlayedElement.innerText = agent.games_played;
            row.appendChild(gamePlayedElement);

            table.appendChild(row);
        }
    });
}