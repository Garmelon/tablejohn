import { MetricsResponse } from "./requests.js";
import { el } from "./util.js";

class Folder {
    metric: string | null = null;
    children: Map<string, Folder> = new Map();

    getOrCreateChild(name: string): Folder {
        let child = this.children.get(name);
        if (child === undefined) {
            child = new Folder();
            this.children.set(name, child);
        }
        return child;
    }

    add(metric: string) {
        let current: Folder = this;
        for (let segment of metric.split("/")) {
            current = current.getOrCreateChild(segment);
        }
        current.metric = metric;
    }

    toHtmlElement(name: string): HTMLElement {
        if (this.children.size > 0) { // Folder
            name = `${name}/`;
            if (this.metric === null) { // Folder without metric
                return el("details", { "class": "no-metric" },
                    el("summary", {}, name),
                    this.childrenToHtmlElements(),
                );
            } else { // Folder with metric
                return el("details", {},
                    el("summary", {},
                        el("input", { "type": "checkbox", "name": this.metric }),
                        " ", name,
                    ),
                    this.childrenToHtmlElements(),
                );
            }
        } else if (this.metric !== null) { // Normal metric
            return el("label", {},
                el("input", { "type": "checkbox", "name": this.metric }),
                " ", name,
            );
        } else { // Metric without metric, should never happen
            return el("label", {}, name);
        }
    }

    childrenToHtmlElements(): HTMLElement {
        let result: HTMLElement = el("ul", {});
        for (let [name, folder] of this.children.entries()) {
            result.append(el("li", {}, folder.toHtmlElement(name)));
        }
        return result;
    }
}

export class Metrics {
    #div: HTMLElement;
    #dataId: number | null = null;

    constructor(div: HTMLElement) {
        this.#div = div;
    }

    getSelected(): Set<string> {
        const selected = new Set<string>();

        const checkedInputs = this.#div.querySelectorAll<HTMLInputElement>("input:checked");
        for (const input of checkedInputs) {
            selected.add(input.name);
        }

        return selected;
    }

    requiresUpdate(dataId: number): boolean {
        // At the moment, updating the metrics results in all <details> tags
        // closing again. To prevent this (as it can be frustrating if you've
        // navigated deep into the metrics hierarchy), we never require updates
        // after the initial update.
        return this.#dataId === null;
        // return this.#dataId === null || this.#dataId < dataId;
    }

    update(response: MetricsResponse) {
        const selected = this.getSelected();

        const folder = new Folder();
        for (const metric of response.metrics) {
            folder.add(metric);
        }

        this.#div.textContent = ""; // Remove children
        if (folder.children.size == 0) {
            this.#div.append("There aren't yet any metrics");
        } else {
            this.#div.append(folder.childrenToHtmlElements());
        }

        const inputs = this.#div.querySelectorAll<HTMLInputElement>("input");
        for (const input of inputs) {
            input.checked = selected.has(input.name);
        }

        this.#dataId = response.dataId;
    }
}
