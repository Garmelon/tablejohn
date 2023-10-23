/**
 * `/graph/metrics` response data.
 */
export type MetricsResponse = {
    dataId: number;
    metrics: string[];
};

/**
 * `/graph/commits` response data.
 */
export type CommitsResponse = {
    graphId: number;
    hashByHash: string[];
    authorByHash: string[];
    committerDateByHash: number[];
    messageByHash: string[];
    parentsByHash: number[][];
};

/**
 * `/graph/measurements` response data.
 */
export type MeasurementsResponse = {
    graphId: number;
    dataId: number;
    measurements: { [key: string]: (number | null)[]; };
};

async function getData<R>(url: string): Promise<R> {
    const response = await fetch(url);
    const data: R = await response.json();
    return data;
}

export async function getMetrics(): Promise<MetricsResponse> {
    return getData("metrics");
}

export async function getCommits(): Promise<CommitsResponse> {
    return getData("commits");
}

export async function getMeasurements(metrics: string[]): Promise<MeasurementsResponse> {
    const params = new URLSearchParams(metrics.map(m => ["metric", m]));
    return getData(`measurements?${params}`);
}
