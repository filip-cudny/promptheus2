const platform = typeof navigator === "undefined" ? "" : navigator.platform || "";
const userAgent = typeof navigator === "undefined" ? "" : navigator.userAgent || "";

export const isMac = /Mac/.test(platform);
export const isLinux = /Linux/i.test(platform) && !/Mac|Win/i.test(platform) && !/Android/i.test(userAgent);
