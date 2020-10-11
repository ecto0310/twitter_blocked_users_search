import { config } from "dotenv";
import Twitter from "twitter";

type Task =
  // Verify account validity
  | {
    type: "authUserId"
  };

type State = {
  // Authenticated User
  authUserId: string;
  // Task queue
  tasks: Task[];
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
  }
}

// Verify account validity
async function authUserId() {
  const res = await client.get("account/verify_credentials", {});
  state.authUserId = res.id_str;
}
