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
