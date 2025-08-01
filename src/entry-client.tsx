/* src/entry-client.tsx */

import 'solid-devtools'
import { mount, StartClient } from "@solidjs/start/client";

mount(() => <StartClient />, document.getElementById("app")!);
