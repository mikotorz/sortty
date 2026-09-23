import { writable } from "svelte/store";

export const selectedRoot = writable<string | null>(null);
