import uPlot from "./uPlot.js";

interface GraphData {
    hashes: string[];
    times: number[];
    measurements: { [key: string]: (number | null)[]; };
}

let opts = {
    title: "HEHE",
    width: 600,
    height: 400,
    series: [
        {},
        {
            label: "wall-clock/build",
            spanGaps: true,
            stroke: "blue",
            width: 1,
        }
    ],
};

let plot = new uPlot(opts, [], document.body);

fetch("data?metric=wall-clock/build")
    .then(r => r.json() as Promise<GraphData>)
    .then(data => {
        console.log(data);
        plot.setData([
            data.times,
            data.measurements["wall-clock/build"]!,
        ]);
    });

// function display(metrics: string[]) {
//     let url = "data" + new URLSearchParams(metrics.map(m => ["metric", m]));
//     fetch(url)
//         .then(r => r.json() as Promise<GraphData>)
//         .then(data => {

//         })
// }
