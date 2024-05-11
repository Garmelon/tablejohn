import { CommitsResponse } from "./requests.js";
import { SECONDS_PER_DAY } from "./util.js";

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
  #committerDatesNormal: Date[] = [];
  #committerDatesDayEquidistant: Date[] = [];

  requiresUpdate(graphId: number): boolean {
    return this.#graphId === null || this.#graphId < graphId;
  }

  update(response: CommitsResponse) {
    console.assert(response.hashByHash.length == response.authorByHash.length);
    console.assert(
      response.hashByHash.length == response.committerDateByHash.length,
    );
    console.assert(response.hashByHash.length == response.summaryByHash.length);

    let commits = this.#loadCommits(response);
    commits = this.#sortCommitsTopologically(commits);
    this.#sortCommitsByCommitterDate(commits);

    // Fill in indexes - "later" is now
    for (const [idx, commit] of commits.entries()) {
      commit.indexByGraph = idx;
    }

    const committerDatesNormal = commits.map((c) => c.committerDate);
    const committerDatesDayEquidistant =
      this.#makeDayEquidistant(committerDatesNormal);

    // To prevent exceptions and other weirdness from messing up our state,
    // we update everything in one go.
    this.#graphId = response.graphId;
    this.#commitsByGraph = commits;
    this.#committerDatesNormal = this.#epochTimesToDates(committerDatesNormal);
    this.#committerDatesDayEquidistant = this.#epochTimesToDates(
      committerDatesDayEquidistant,
    );
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
   * A reverse post-order DFS is a topological sort, so that is what this
   * function implements.
   */
  #sortCommitsTopologically(commits: Commit[]): Commit[] {
    const visited: Set<string> = new Set();
    const visiting: Commit[] = commits.filter((c) => c.parents.length == 0);

    const sorted: Commit[] = [];

    while (visiting.length > 0) {
      const commit = visiting.at(-1)!;
      if (visited.has(commit.hash)) {
        visiting.pop();
        sorted.push(commit);
        continue;
      }

      for (const child of commit.children) {
        if (!visited.has(child.hash)) {
          visiting.push(child);
        }
      }

      visited.add(commit.hash);
    }

    sorted.reverse();

    console.assert(visited.size === commits.length);
    console.assert(visiting.length === 0);
    console.assert(sorted.length === commits.length);
    return sorted;
  }

  /**
   * Assumes the times are sorted.
   */
  #makeDayEquidistant(times: number[]): number[] {
    const days: { day: number; amount: number }[] = [];
    for (const time of times) {
      const day = time % SECONDS_PER_DAY;
      const prev = days.at(-1);
      if (prev === undefined || prev.day !== day) {
        days.push({ day, amount: 1 });
      } else {
        prev.amount++;
      }
    }

    const result: number[] = [];
    for (const day of days) {
      const secondsPerCommit = SECONDS_PER_DAY / day.amount;
      for (let i = 0; i < day.amount; i++) {
        const time = day.day * SECONDS_PER_DAY + secondsPerCommit * (i + 0.5);
        result.push(time);
      }
    }

    return result;
  }

  #epochTimesToDates(times: number[]): Date[] {
    return times.map((t) => new Date(1000 * t));
  }
}
