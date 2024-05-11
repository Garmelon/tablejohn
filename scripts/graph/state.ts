import { Commits } from "./commits.js";
import { Metrics } from "./metrics.js";
import { getCommits, getMetrics } from "./requests.js";

export class State {
  #latestGraphId: number = -Infinity;
  #latestDataId: number = -Infinity;

  #metrics: Metrics;
  #commits: Commits = new Commits();

  #requestingMetrics: boolean = false;
  #requestingCommits: boolean = false;

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

    if (this.#commits.requiresUpdate(this.#latestGraphId)) {
      this.#requestCommits();
    }
  }

  async #requestMetrics() {
    if (this.#requestingMetrics) return;
    console.log("Requesting metrics");
    try {
      this.#requestingMetrics = true;
      const response = await getMetrics();
      this.#updateDataId(response.dataId);
      this.#metrics.update(response);
      this.update();
    } finally {
      this.#requestingMetrics = false;
    }
  }

  async #requestCommits() {
    if (this.#requestingCommits) return;
    console.log("Requesting commits");
    try {
      this.#requestingCommits = true;
      const response = await getCommits();
      this.#updateGraphId(response.graphId);
      this.#commits.update(response);
      this.update();
    } finally {
      this.#requestingCommits = false;
    }
  }
}
