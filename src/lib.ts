import * as fs from "fs";
import { chunksOf } from 'fp-ts/Array'
import { config } from "dotenv";
import Twitter from "twitter";
import * as readline from "readline"

type Task =
  // Verify account validity
  | {
    type: "authUserId"
  }
  // Fetch users at distance 1
  | {
    type: "fetchDis1Users";
    direction: string;
    cursor: string;
  }
  // Fetch users at distance 2
  | {
    type: "fetchDis2Users";
    direction: string;
    id: string,
    cursor: string;
  }
  // End of to fetch users
  | {
    type: "endFetchUsers";
  }
  // Check blocked
  | {
    type: "checkBlocked";
    ids: string[];
  }
  // End of to check blocked
  | {
    type: "endCheckBlocked"
  };


type User = {
  id: string;
  name: string;
  screen_name: string;
  friend: string[];
};

type State = {
  // Authenticated User
  authUserId: string;
  // List of blocked users
  blockedUsers: Map<string, User>;
  // Task queue
  tasks: Task[];
  // List of error tasks
  errorTasks: Task[];
  // List of users at distance 1
  dis1UsersId: Set<string>;
  // List of users at distance 2
  dis2UsersId: Map<string, Set<string>>;
  // Bar number
  barNumber: NodeJS.Timeout | null;
  // Bar count
  barCount: number;
  // Rerun flag
  rerun: boolean;
};

let client: Twitter;
let state: State;

// Initialization phase
export function init() {
  config();
  client = new Twitter({
    consumer_key: process.env["consumer_key"]!,
    consumer_secret: process.env["consumer_secret"]!,
    access_token_key: process.env["access_token"]!,
    access_token_secret: process.env["access_token_secret"]!
  });
  loadState();
  state.tasks.unshift({ type: "authUserId" });
  state.barNumber = setInterval(progressBar, 1000);
}

// Display progress bar
function progressBar() {
  state.barCount = (state.barCount + 1) % 4;
  readline.clearLine(process.stdout, 0);
  if (state.tasks.length === 0) {
    console.log("Succeed all process.\r");
    clearInterval(state.barNumber!);
  }
  else {
    process.stdout.write("Running tasks..." + "|/-\\".charAt(state.barCount) + " (Remaining tasks: " + state.tasks.length + ")\r");
  }
}

// Main process
export async function main() {
  while (state.tasks.length !== 0) {
    await progress();
    saveState();
  }
}

// Process
async function progress() {
  const task = state.tasks.shift();
  if (task === undefined) {
    throw new Error("Tasks is empty");
  }
  switch (task.type) {
    // Verify account validity
    case "authUserId": {
      await authUserId();
      return;
    }
    // Fetch users at distance 1
    case "fetchDis1Users": {
      await sleep(60 * 1000);
      await fetchDis1Users(task);
      return;
    }
    // Fetch users at distance 2
    case "fetchDis2Users": {
      await sleep(60 * 1000);
      await fetchDis2Users(task);
      return;
    }
    // End of to fetch users
    case "endFetchUsers": {
      endFetchUsers();
      return;
    }
    // Check blocked
    case "checkBlocked": {
      await sleep(1 * 1000);
      checkBlocked(task);
      return;
    }
    // End of to check blocked
    case "endCheckBlocked": {
      endCheckBlocked();
      return;
    }
  }
}

// Verify account validity
async function authUserId() {
  const res = await client.get("account/verify_credentials", {});
  if (state.rerun) {
    if (state.authUserId !== res.id_str) {
      console.log("Data file does not match the user.");
      process.exit(0);
    }
  }
  state.authUserId = res.id_str;
}

// Fetch users at distance 1
async function fetchDis1Users(task: Task) {
  if (task.type !== "fetchDis1Users") {
    throw new Error("Task mismatch");
  }
  // Fetch follow at distance 1
  if (task.direction === "follow") {
    const res = await client.get("friends/ids", {
      user_id: state.authUserId,
      cursor: task.cursor,
      stringify_ids: "true",
      count: "5000"
    });

    const ids: string[] = res.ids;
    ids.forEach(id => state.dis1UsersId.add(id));

    // End of fetch follow at distance 1
    if (res.next_cursor_str === "0") {
      state.tasks.unshift({
        type: "fetchDis1Users",
        direction: "follower",
        cursor: "-1"
      });
    }
    // Continue to fetch follow at distance 1
    else {
      state.tasks.unshift({
        type: "fetchDis1Users",
        direction: "follow",
        cursor: res.next_cursor_str
      });
    }
  }
  // Fetch follower at distance 1
  else if (task.direction === "follower") {
    const res = await client.get("followers/ids", {
      user_id: state.authUserId,
      cursor: task.cursor,
      stringify_ids: "true",
      count: "5000"
    });

    const ids: string[] = res.ids;
    ids.forEach(id => state.dis1UsersId.add(id));

    // End of fetch follower at distance 1
    if (res.next_cursor_str === "0") {
      state.dis1UsersId.forEach(id => state.tasks.unshift({
        type: "fetchDis2Users",
        direction: "follow",
        id: id,
        cursor: "-1"
      }));
    }
    // Continue to fetch follower at distance 1
    else {
      state.tasks.unshift({
        type: "fetchDis1Users",
        direction: "follower",
        cursor: res.next_cursor_str
      });
    }
  }
}

// Fetch users at distance 2
async function fetchDis2Users(task: Task) {
  if (task.type !== "fetchDis2Users") {
    throw new Error("Task mismatch");
  }
  try {
    // Fetch follow at distance 1
    if (task.direction === "follow") {
      const res = await client.get("friends/ids", {
        user_id: task.id,
        cursor: task.cursor,
        stringify_ids: "true",
        count: "5000"
      });

      const ids: string[] = res.ids;
      let users = new Set<string>();
      if (state.dis2UsersId.get(task.id) !== undefined) {
        users = state.dis2UsersId.get(task.id)!;
      }
      ids.forEach(id => users.add(id));
      state.dis2UsersId.set(task.id, users);

      // End of fetch follow at distance 2
      if (res.next_cursor_str === "0") {
        state.tasks.unshift({
          type: "fetchDis2Users",
          direction: "follower",
          id: task.id,
          cursor: "-1"
        });
      }
      // Continue to fetch follow at distance 2
      else {
        state.tasks.unshift({
          type: "fetchDis2Users",
          direction: "follow",
          id: task.id,
          cursor: res.next_cursor_str
        });
      }
    }
    // Fetch follower at distance 2
    else if (task.direction === "follower") {
      const res = await client.get("followers/ids", {
        user_id: task.id,
        cursor: task.cursor,
        stringify_ids: "true",
        count: "5000"
      });

      const ids: string[] = res.ids;
      let users = new Set<string>();
      if (state.dis2UsersId.get(task.id) !== undefined) {
        users = state.dis2UsersId.get(task.id)!;
      }
      ids.forEach(id => users.add(id));
      state.dis2UsersId.set(task.id, users);

      // End of fetch follower at distance 2
      if (res.next_cursor_str === "0") {
      }
      // Continue to fetch follower at distance 2
      else {
        state.tasks.unshift({
          type: "fetchDis2Users",
          direction: "follower",
          id: task.id,
          cursor: res.next_cursor_str
        });
      }
    }
    // Error handling
  } catch (e) {
    state.errorTasks.push(task);
    if (state.dis2UsersId.get(task.id) !== undefined) {
      state.dis2UsersId.set(task.id, new Set<string>());
    }
    return;
  }
}

// End of to fetch users
function endFetchUsers() {
  let users = new Set<string>();
  state.dis2UsersId.forEach(s => s.forEach(t => users.add(t)));
  chunksOf(100)(Array.from(users)).forEach(chunks => {
    state.tasks.unshift({
      type: "checkBlocked",
      ids: chunks
    });
  });
}

// Check blocked
async function checkBlocked(task: Task) {
  if (task.type !== "checkBlocked") {
    throw new Error("Task mismatch");
  }
  const res = await client.get("users/lookup", {
    user_id: task.ids.join(","),
    include_blocked_by: "true"
  });
  const users: {
    id_str: string;
    name: string;
    screen_name: string;
    blocked_by: boolean;
  }[] = res as any;
  users.forEach(user => {
    if (user.blocked_by) {
      state.blockedUsers.set(user.id_str, {
        id: user.id_str,
        name: user.name,
        screen_name: user.screen_name,
        friend: []
      });
    }
  });
}

// End of to check blocked
function endCheckBlocked() {
  state.blockedUsers.forEach(user => {
    state.dis2UsersId.forEach((friends, id) => {
      if (friends.has(user.id)) {
        user.friend.push(id);
      }
    });
    state.blockedUsers.set(user.id, user);
  });
}

// Export format
type SaveData = {
  authUserId: string;
  blockedUsers: [string, User][];
  tasks: Task[];
  errorTasks: Task[];
  dis1UsersId: string[];
  dis2UsersId: [string, string[]][];
};

// Load state
function loadState() {
  try {
    let saveData: SaveData = JSON.parse(fs.readFileSync("data.json", { encoding: "utf8" }));
    state = {
      authUserId: saveData.authUserId,
      blockedUsers: new Map(
        saveData.blockedUsers.map(([k, v]) => [k, v])
      ),
      tasks: saveData.tasks,
      errorTasks: saveData.errorTasks,
      dis1UsersId: new Set(saveData.dis1UsersId),
      dis2UsersId: new Map(
        saveData.dis2UsersId.map(([k, v]) => [k, new Set(v)])
      ),
      barNumber: null,
      barCount: 0,
      rerun: true
    };
  } catch {
    state = {
      authUserId: "",
      blockedUsers: new Map(),
      tasks: [
        {
          type: "fetchDis1Users",
          direction: "follow",
          cursor: "-1"
        },
        { type: "endFetchUsers" },
        { type: "endCheckBlocked" }
      ],
      errorTasks: [],
      dis1UsersId: new Set(),
      dis2UsersId: new Map(),
      barNumber: null,
      barCount: 0,
      rerun: false
    };
  }
}

// Save state
function saveState() {
  let saveData: SaveData = {
    authUserId: state.authUserId,
    blockedUsers: Array.from(state.blockedUsers).map(([k, v]) => [k, v]),
    tasks: state.tasks,
    errorTasks: state.errorTasks,
    dis1UsersId: Array.from(state.dis1UsersId),
    dis2UsersId: Array.from(state.dis2UsersId).map(([k, v]) => [
      k,
      Array.from(v)
    ])
  };
  fs.writeFileSync("data.json", JSON.stringify(saveData));
}

// Sleep function
export function sleep(msec: number): Promise<void> {
  return new Promise(resolve => {
    setTimeout(() => {
      resolve();
    }, msec);
  });
}
