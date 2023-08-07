const COUNT = document.getElementById("count")!;
const QUEUE = document.getElementById("queue")!;
const REFRESH_SECONDS = 10;

function update() {
    fetch("table")
        .then(response => response.text())
        .then(text => {
            QUEUE.innerHTML = text;
            let count = QUEUE.querySelectorAll("tbody tr").length;
            COUNT.textContent = String(count);
            document.title = document.title.replace(/^queue \(\d+\)/, `queue (${count})`);
        });
}

setInterval(update, REFRESH_SECONDS * 1000);
