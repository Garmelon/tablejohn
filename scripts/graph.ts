import uPlot from "./uPlot.js";

/*

Design goals and reasoning
==========================

The graph should be fast. This requires a bit of careful thinking around data
formats and resource usage.

My plan is to get as far as possible without any sort of pagination or range
limits. The plot should always display data for the repo's entire history
(unless zoomed in). This will force me to optimize the entire pipeline. If
the result is not fast enough, I can still add in range limits.

uPlot is pretty fast at rendering large amounts of data points. It should be
able to handle medium-sized git repos (tens of thousands of commits)
displaying multiple metrics with no issues. The issue now becomes retrieving
the data from the server.

Since the graph should support thousands of metrics, it can't simply fetch
all values for all metrics upfront. Instead, it must fetch metrics as the
user selects them. It follows that when fetching a metric,

1. the server should have little work to do,
2. the amount of data sent over the network should be small, and
3. the client should have little work to do.

The costs when initially loading the graph may be higher since it happens
less frequently. We can fetch some more data and do some preprocessing to
improve performance while interacting with the graph.

Since we fetch data across multiple requests, we need some way to detect if
all the data we have is consistent (at least in cases where things might
otherwise break).

Implementation
==============

The data for the graph consists of three main parts:

1. The names of the available metrics
2. The commit graph metadata (commit hashes, parents, authors, dates)
3. The values and units for the selected metrics

Data consistency
----------------

Each response by the server includes a graph id and a data id.

The graph id is incremented when the commit graph structure changes. Responses
to 2. and 3. MUST have the same graph id. When they don't, the client must
re-fetch those resources with a smaller graph id. Responses with different graph
ids MUST NOT be combined.

The data id is incremented when data changes (usually because a new run is
added). All responses to 1. and 3. SHOULD have the same data id. Responses with
different data id MAY be combined.

Data flow
---------

┌────────────────┐   ┌─────────────────────┐    ┌────────────────┐
│ /graph/metrics │   │ /graph/measurements │    │ /graph/commits │
└──┬─────────────┘   └──┬──────────────────┘    └──────────┬─────┘
   │      Server        │                                  │
───┼────────────────────┼─────────Requests─────────────────┼──────
   │                    │                                  │
┌──▼─────┐    ┌─────────▼────┐  ┌─────────────────┐  ┌─────▼─────┐
│ metric │    │ measurements │  │ permute by-hash ◄──┤commit info│
│ names  │    │   by-hash    │  │ to by-date-topo │  │  by-hash  │
└──┬─────┘    └─────────┬────┘  └──┬──────────────┘  └─────┬─────┘
   │                    │          │                       │
   │          ┌─────────▼────┐     │              ┌────────▼─────┐
   │    ┌─────► measurements ◄─────┘              │ commit info  │
   │    │     │ by-date-topo │                    │ by-date-topo │
   │    │     └─────────┬────┘                    └────────┬─────┘
   │    │               │          ┌────────┐              │
   │    │     State     └──────────►  data  ◄──────────────┘
   │    │                          │ series │
   │    │                          └───┬────┘
   │    │                              │
───┼────┼──────────────────────────────┼──────────────────────────
   │    │                              │
┌──▼────┴──┐  ┌─────────────────┐  ┌───▼──┐
│  metric  │  │ day-equidistant ├──► plot │           UI
│ selector │  │     checkbox    │  └──────┘
└──────────┘  └─────────────────┘

*/

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
    // "#fabed4", // Pink
    "#469990", // Teal
    // "#dcbeff", // Lavender
    "#9A6324", // Brown
    // "#fffac8", // Beige
    "#800000", // Maroon
    // "#aaffc3", // Mint
    // "#808000", // Olive
    // "#ffd8b1", // Apricot
    "#000075", // Navy
    "#a9a9a9", // Grey
    // "#ffffff", // White
    "#000000", // Black
];

// Initialization

const plotDiv = document.getElementById("plot")!;
const metricsDiv = document.getElementById("metrics")!;
let plot: uPlot | null = null;
