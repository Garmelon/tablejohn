import { Metrics } from "./metrics.js";
import { getMetrics } from "./requests.js";

export class State {
    #latestGraphId: number = -Infinity;
    #latestDataId: number = -Infinity;

    #metrics: Metrics;

    #requestingNewMetrics: boolean = false;

    // commits (with graph id and data id)
    // raw measurements (with graph id and data id)
    // processed measurements (with graph id and data id)

    constructor(metrics: Metrics) {
        this.#metrics = metrics;
    }

    /**
     * Update state and plot and request new data if necessary. Tries to match
     * the user's wishes as closely as possible.
     *
     * This function is idempotent.
     */
    update() {
        // TODO Invalidate and update data
        // TODO Update graph
        this.#requestDataWhereNecessary();
    }

    //////////////////////////////////
    // Requesting and updating data //
    //////////////////////////////////

    #updateDataId(dataId: number) {
        if (dataId > this.#latestDataId) {
            this.#latestDataId = dataId;
        }
    }

    #updateGraphId(graphId: number) {
        if (graphId > this.#latestGraphId) {
            this.#latestGraphId = graphId;
        }
    }

    #requestDataWhereNecessary() {
        if (this.#metrics.requiresUpdate(this.#latestDataId)) {
            this.#requestMetrics();
        }
    }

    async #requestMetrics() {
        if (this.#requestingNewMetrics) return;
        console.log("Requesting new metrics");
        try {
            this.#requestingNewMetrics = true;
            const response = await getMetrics();
            this.#updateDataId(response.dataId);
            this.#metrics.update(response);
            this.update();
        } finally {
            this.#requestingNewMetrics = false;
        }
    }
}
