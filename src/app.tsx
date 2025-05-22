/* src/app.tsx */

import { createSignal } from "solid-js";
import "./app.css";

export default function App() {
  const [count, setCount] = createSignal(0);

  return (
    <main class="p-4">
      <h1 class="text-3xl font-bold underline text-sky-600 dark:text-sky-300">
        Hello world!
      </h1>
      <button class="increment mt-4 bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded" onClick={() => setCount(count() + 1)} type="button">
        Clicks: {count()}
      </button>
      <p class="mt-4">
        Visit{" "}
        <a href="https://start.solidjs.com" target="_blank" class="text-blue-600 hover:underline dark:text-blue-400">
          start.solidjs.com
        </a>{" "}
        to learn how to build SolidStart apps.
      </p>
    </main>
  );
}