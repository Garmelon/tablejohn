import { updateMetricsDiv } from "./metrics.js";
import { MetricsResponse, getMetrics } from "./requests.js";

export class State {
    #metricsDiv: HTMLElement;

    #updating: boolean = false;
    #metrics: MetricsResponse | null = null;

    constructor(metricsDiv: HTMLElement) {
        this.#metricsDiv = metricsDiv;
    }

    /**
     * Look at current state and try to change it so that it represents what the
     * user wants.
     *
     * This function is idempotent.
     */
    async update() {
        if (this.#updating) {
            return;
        }
        try {
            await this.#update_impl();
        } finally {
            this.#updating = false;
        }
    }

    async #update_impl() {
        this.#update_metrics();
    }

    async #update_metrics() {
        this.#metrics = await getMetrics();
        if (this.#metrics === null) { return; }
        updateMetricsDiv(this.#metricsDiv, this.#metrics.metrics);
    }
}
