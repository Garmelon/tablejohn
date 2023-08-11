const COUNT = document.getElementById("count")!;
const INNER = document.getElementById("inner")!;
const REFRESH_SECONDS = 10;

function update() {
    fetch("inner")
        .then(response => response.text())
        .then(text => {
            INNER.innerHTML = text;
            let count = INNER.querySelector<HTMLElement>("#queue")?.dataset["count"]!;
            document.title = document.title.replace(/^queue \(\d+\)/, `queue (${count})`);
        });
}

setInterval(update, REFRESH_SECONDS * 1000);