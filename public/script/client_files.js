function onLoad() {
    fetch("/api/list_files").then(res => res.json()).then(res => {
        let main = document.getElementsByTagName("main")[0];

        for (lang in res) {
            let container = document.createElement("div");
            container.classList.add("language-container");

            let heading = document.createElement("h2");
            heading.classList.add("language-name");
            heading.innerText = lang;
            container.appendChild(heading);

            let table = document.createElement("table");
            table.classList.add("file-table");
            container.appendChild(table);

            let headingRow = document.createElement("tr");
            for (title of ["Name", "Description"]) {
                let e = document.createElement("th");
                e.innerText = title;

                if (title == "Name") {
                    e.style.width = "300px";
                }

                headingRow.appendChild(e);
            }
            table.appendChild(headingRow);

            res[lang].sort(function(a, b) {
                if (a.display < b.display) return -1;
                if (a.display > b.display) return 1;
                return 0;
            });

            for (file of res[lang]) {
                let row = document.createElement("tr");

                let name = document.createElement("td");
                let nameLink = document.createElement("a");
                nameLink.href = `/client_files/${encodeURIComponent(lang)}/${encodeURIComponent(file.name)}/`;
                nameLink.style.color = "black";
                nameLink.innerText = file.display;
                name.appendChild(nameLink);
                row.appendChild(name);

                let description =document.createElement("td");
                description.innerText = file.description;
                row.appendChild(description);

                table.appendChild(row);
            }

            main.appendChild(container);
        }
    });
}