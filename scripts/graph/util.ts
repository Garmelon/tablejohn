/**
 * Create an {@link HTMLElement}.
 */
export function el(name: string, attributes: { [key: string]: string; }, ...children: (string | Node)[]) {
    let element = document.createElement(name);
    for (let [name, value] of Object.entries(attributes)) {
        element.setAttribute(name, value);
    }
    element.append(...children);
    return element;
}

export const SECONDS_PER_DAY = 24 * 60 * 60;
