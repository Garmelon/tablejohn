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

export function updateMetricsDiv(div: HTMLElement, metrics: string[]) {
    let folder = new Folder();
    for (let metric of metrics) {
        folder.add(metric);
    }

    div.textContent = ""; // Remove children
    div.append(folder.childrenToHtmlElements());
}
