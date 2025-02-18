import { redirect } from "@sveltejs/kit";

export const load = () => {
  const featureTokensEnabled = import.meta.env.VITE_FEATURE_TOKENS === "true";

  if (!featureTokensEnabled) {
    throw redirect(302, "/");
  }
};
