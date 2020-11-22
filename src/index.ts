import { init, main, sleep } from "./lib";

(async () => {
  let finish = false;
  while (!finish) {
    try {
      init();
      await main();
      finish = true;
    } catch (e) {
      console.log(e);
      await sleep(60 * 1000);
    }
  }
})();
