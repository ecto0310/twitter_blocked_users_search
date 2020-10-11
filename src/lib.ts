import { config } from "dotenv";
import Twitter from "twitter";

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
  };

type State = {
  // Authenticated User
  authUserId: string;
  // Task queue
  tasks: Task[];
  // List of users at distance 1
  dis1UsersId: Set<string>;
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
}

// Main process
export async function main() {
  while (state.tasks.length !== 0) {
    await progress();
  }
  console.log("Succeed all process.");
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
  }
}

// Verify account validity
async function authUserId() {
  const res = await client.get("account/verify_credentials", {});
  state.authUserId = res.id_str;
  state.tasks.push({
    type: "fetchDis1Users",
    direction: "follow",
    cursor: "-1"
  });
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

// Sleep function
function sleep(msec: number): Promise<void> {
  return new Promise(resolve => {
    setTimeout(() => {
      resolve();
    }, msec);
  });
}
