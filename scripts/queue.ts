const INNER = document.getElementById("inner")!;
const REFRESH_SECONDS = 10;

function update() {
    fetch("inner")
        .then(response => response.text())
        .then(text => {
            INNER.innerHTML = text;
            let count = document.getElementById("queue")?.dataset["count"]!;
            document.title = document.title.replace(/^queue \(\S+\)/, `queue (${count})`);
        });
}

setInterval(update, REFRESH_SECONDS * 1000);
