import { getSupabase } from "./supabase";

export async function submitWaitlist(
  email: string
): Promise<{ success: boolean; message: string }> {
  const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
  if (!emailRegex.test(email)) {
    return { success: false, message: "Please enter a valid email address." };
  }

  const supabase = getSupabase();
  if (!supabase) {
    return { success: false, message: "Something went wrong. Please try again." };
  }

  const { error } = await supabase
    .from("waitlist")
    .insert({ email, source: "website" });

  if (error) {
    // Unique constraint violation (duplicate email)
    if (error.code === "23505") {
      return { success: true, message: "You're already on the list!" };
    }
    return { success: false, message: "Something went wrong. Please try again." };
  }

  return { success: true, message: "You're on the list! We'll be in touch." };
}
