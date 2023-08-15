import uPlot from "./uPlot.js";

// https://sashamaps.net/docs/resources/20-colors/
// Related: https://en.wikipedia.org/wiki/Help:Distinguishable_colors
const COLORS = [
    "#e6194B", // Red
    "#3cb44b", // Green
    "#ffe119", // Yellow
    "#4363d8", // Blue
    "#f58231", // Orange
    // "#911eb4", // Purple
    "#42d4f4", // Cyan
    "#f032e6", // Magenta
    // "#bfef45", // Lime
    "#fabed4", // Pink
    "#469990", // Teal
    "#dcbeff", // Lavender
    "#9A6324", // Brown
    "#fffac8", // Beige
    "#800000", // Maroon
    "#aaffc3", // Mint
    // "#808000", // Olive
    "#ffd8b1", // Apricot
    "#000075", // Navy
    "#a9a9a9", // Grey
    // "#ffffff", // White
    "#000000", // Black
];

interface GraphData {
    hashes: string[];
    times: number[];
    measurements: { [key: string]: (number | null)[]; };
}

function update_plot_with_data(data: GraphData) {
    let series: uPlot.Series[] = [{}];
    let values: uPlot.AlignedData = [data.times];

    for (const [i, metric] of Object.keys(data.measurements).sort().entries()) {
        series.push({
            label: metric,
            spanGaps: true,
            stroke: COLORS[i % COLORS.length],
        });
        values.push(data.measurements[metric]!);
    }

    const opts: uPlot.Options = {
        title: "Measurements",
        width: 600,
        height: 400,
        series,
    };

    plot?.destroy();
    plot = new uPlot(opts, values, plot_div);
}

async function update_plot_with_metrics(metrics: string[]) {
    const url = "data?" + new URLSearchParams(metrics.map(m => ["metric", m]));
    const response = await fetch(url);
    const data: GraphData = await response.json();
    update_plot_with_data(data);
}

function find_selected_metrics(): string[] {
    const inputs = metrics_div.querySelectorAll<HTMLInputElement>('input[type="checkbox"]');

    let metrics: string[] = [];
    for (const input of inputs) {
        if (input.checked) {
            metrics.push(input.name);
        }
    }
    return metrics;
};

async function update_plot() {
    const metrics = find_selected_metrics();
    if (metrics.length > 0) {
        await update_plot_with_metrics(metrics);
    } else {
        update_plot_with_data({
            hashes: [],
            times: [],
            measurements: {},
        });
    }
}

// Initialization

const plot_div = document.getElementById("plot")!;
const metrics_div = document.getElementById("metrics")!;
let plot: uPlot | null = null;

for (const input of metrics_div.querySelectorAll<HTMLInputElement>('input[type="checkbox"]')) {
    input.addEventListener("change", update_plot);
}

update_plot();
