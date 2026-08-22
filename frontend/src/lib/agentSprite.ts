import greenDrive1 from "../assets/sprites/robot_greenDrive1.png";
import greenDrive2 from "../assets/sprites/robot_greenDrive2.png";
import greenHurt from "../assets/sprites/robot_greenHurt.png";
import blueDrive1 from "../assets/sprites/robot_blueDrive1.png";
import blueDrive2 from "../assets/sprites/robot_blueDrive2.png";
import blueHurt from "../assets/sprites/robot_blueHurt.png";
import yellowDrive1 from "../assets/sprites/robot_yellowDrive1.png";
import yellowDrive2 from "../assets/sprites/robot_yellowDrive2.png";
import yellowHurt from "../assets/sprites/robot_yellowHurt.png";
import redDrive1 from "../assets/sprites/robot_redDrive1.png";
import redDrive2 from "../assets/sprites/robot_redDrive2.png";
import redHurt from "../assets/sprites/robot_redHurt.png";
import redDamage2 from "../assets/sprites/robot_redDamage2.png";

export type SpriteColor = "green" | "blue" | "yellow" | "red";

const COLORS: SpriteColor[] = ["green", "blue", "yellow", "red"];

const SPRITES: Record<SpriteColor, { drive1: string; drive2: string; hurt: string }> = {
  green: { drive1: greenDrive1, drive2: greenDrive2, hurt: greenHurt },
  blue: { drive1: blueDrive1, drive2: blueDrive2, hurt: blueHurt },
  yellow: { drive1: yellowDrive1, drive2: yellowDrive2, hurt: yellowHurt },
  red: { drive1: redDrive1, drive2: redDrive2, hurt: redHurt },
};

/** The one non-color-specific pose used for the misuse-alert icon in the
 * activity feed — reused regardless of which agent triggered it, since
 * that event isn't about a specific agent's own sprite. */
export const misuseIcon = redDamage2;

/** Deterministic so the same agent always gets the same character across
 * renders/reloads — not random, and not configurable (there's nothing in
 * the domain model to base a "real" choice on, so a stable hash of the
 * id is the honest option over either randomness or a fake preference). */
export function spriteColorFor(agentId: string): SpriteColor {
  let hash = 0;
  for (let i = 0; i < agentId.length; i++) {
    hash = (hash * 31 + agentId.charCodeAt(i)) >>> 0;
  }
  return COLORS[hash % COLORS.length];
}

export function spriteFor(agentId: string) {
  return SPRITES[spriteColorFor(agentId)];
}
