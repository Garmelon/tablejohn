/**
 * `/graph/metrics` response data.
 */
export type MetricsResponse = {
    // data_id: number; // TODO Uncomment

    metrics: string[];
};

/**
 * `/graph/commits` response data.
 */
export type CommitsResponse = {
    // graph_id: number; // TODO Uncomment

    hash_by_hash: string[];
    author_by_hash: number[];
    committer_date_by_hash: string[];
    message_by_hash: string[];
    parents: [string, string][];
};

/**
 * `/graph/measurements` response data.
 */
export type MeasurementsResponse = {
    // graph_id: number; // TODO Uncomment
    // data_id: number; // TODO Uncomment

    measurements: { [key: string]: (number | null)[]; };
};

/**
 * Request different kinds of data from the server.
 *
 * This class has two main purposes:
 *
 * 1. Providing a nice interface for requesting data from the server
 * 2. Preventing sending the same request again while still waiting for the server
 */
export class Requests {
    #requesting_metrics: Promise<MetricsResponse> | null = null;
    #requesting_commits: Promise<CommitsResponse> | null = null;
    #requesting_measurements: Map<string, Promise<MeasurementsResponse>> = new Map();

    async #request_data<R>(url: string): Promise<R> {
        let response = await fetch(url);
        let data: R = await response.json();
        return data;
    }

    async get_metrics(): Promise<MetricsResponse | null> {
        if (this.#requesting_metrics !== null) {
            try {
                return await this.#requesting_metrics;
            } catch (error) {
                return null;
            }
        }

        this.#requesting_metrics = this.#request_data<MetricsResponse>("metrics");
        try {
            return await this.#requesting_metrics;
        } catch (error) {
            console.error("Could not get metrics:", error);
            return null;
        } finally {
            this.#requesting_metrics = null;
        }
    }
}
