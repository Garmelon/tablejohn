import { CommitsResponse } from "./requests";

type Commit = {
    indexByHash: number;
    indexByGraph: number;
    hash: string;
    parents: Commit[];
    children: Commit[];
    author: string;
    committerDate: number;
    summary: string;
};

export class Commits {
    #graphId: number | null = null;
    #commitsByGraph: Commit[] = [];

    requiresUpdate(graphId: number): boolean {
        return this.#graphId === null || this.#graphId < graphId;
    }

    update(response: CommitsResponse) {
        console.assert(response.hashByHash.length == response.authorByHash.length);
        console.assert(response.hashByHash.length == response.committerDateByHash.length);
        console.assert(response.hashByHash.length == response.summaryByHash.length);

        let commits = this.#loadCommits(response);
        commits = this.#sortCommitsTopologically(commits);
        this.#sortCommitsByCommitterDate(commits);

        // Fill in indexes - "later" is now
        for (const [idx, commit] of commits.entries()) {
            commit.indexByGraph = idx;
        }

        this.#graphId = response.graphId;
        this.#commitsByGraph = commits;
    }

    #loadCommits(response: CommitsResponse): Commit[] {
        const commits = new Map<string, Commit>();
        const commitsByHash = [];

        for (const [idx, hash] of response.hashByHash.entries()) {
            const commit = {
                indexByHash: idx,
                indexByGraph: NaN, // Filled in later
                hash,
                parents: [],
                children: [],
                author: response.authorByHash[idx]!,
                committerDate: response.committerDateByHash[idx]!,
                summary: response.summaryByHash[idx]!,
            };
            commits.set(hash, commit);
            commitsByHash.push(commit);
        }

        // Fill in parents and children
        for (const [childIdx, parentIdx] of response.childParentIndexPairs) {
            const childHash = response.hashByHash[childIdx]!;
            const parentHash = response.hashByHash[parentIdx]!;

            const child = commits.get(childHash)!;
            const parent = commits.get(parentHash)!;

            child.parents.push(parent);
            parent.children.push(child);
        }

        return commitsByHash;
    }

    #sortCommitsByCommitterDate(commits: Commit[]) {
        commits.sort((a, b) => a.committerDate - b.committerDate);
    }

    /**
     * Sort commits topologically such that parents come before their children.
     *
     * Assumes that there are no duplicated commits anywhere.
     *
     * The algorithm used is a version of [Kahn's algorithm][0] that starts at the
     * nodes with no parents. It uses a stack for the set of parentless nodes,
     * meaning the resulting commit order is depth-first-y, not breadth-first-y.
     * For example, this commit graph (where children are ordered top to bottom)
     * results in the order `A, B, C, D, E, F` and not an interleaved order like
     * `A, B, D, C, E, F` (which a queue would produce):
     *
     * ```text
     * A - B - C
     *  \       \
     *   D - E - F
     * ```
     *
     * [0]: https://en.wikipedia.org/wiki/Topological_sorting#Kahn's_algorithm
     */
    #sortCommitsTopologically(commits: Commit[]): Commit[] {
        // Track which unvisited parents are left for each commit
        const childParentMap: Map<string, Set<string>> = new Map();
        for (const commit of commits) {
            childParentMap.set(commit.hash, new Set(commit.parents.map(p => p.hash)));
        }

        // Stack of parentless commits
        const parentless = commits.filter(c => c.parents.length == 0);

        const sorted: Commit[] = [];
        while (parentless.length > 0) {
            // Visit commit
            const commit = parentless.pop()!;
            sorted.push(commit);

            for (const child of commit.children) {
                const parents = childParentMap.get(child.hash)!;
                parents.delete(commit.hash);
                if (parents.size == 0) {
                    parentless.push(child);
                }
            }
        }

        for (const [child, parents] of childParentMap.entries()) {
            console.assert(parents.size == 0, child, "still has parents");
        }
        console.assert(parentless.length == 0);
        console.assert(commits.length == sorted.length, "topo sort changed commit amount");
        return sorted;
    }
}
