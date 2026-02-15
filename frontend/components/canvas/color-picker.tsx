"use client";

import { PICO8_PALETTE, PICO8_NAMES } from "@/types/canvas";

interface ColorPickerProps {
  onSelect: (colorIndex: number) => void;
  onCancel: () => void;
  disabled?: boolean;
}

export function ColorPicker({ onSelect, onCancel, disabled }: ColorPickerProps) {
  return (
    <div className="rounded-xl border border-neutral-700 bg-neutral-900 p-3 shadow-2xl">
      <div className="mb-2 text-xs font-medium text-neutral-400">
        Pick a color
      </div>
      <div className="grid grid-cols-4 gap-1.5">
        {PICO8_PALETTE.map((hex, i) => (
          <button
            key={i}
            onClick={() => onSelect(i)}
            disabled={disabled}
            title={PICO8_NAMES[i]}
            className="h-8 w-8 rounded-md border-2 border-transparent transition-all hover:scale-110 hover:border-white disabled:opacity-30"
            style={{ backgroundColor: hex }}
          />
        ))}
      </div>
      <button
        onClick={onCancel}
        className="mt-2 w-full rounded-md py-1 text-xs text-neutral-500 hover:text-neutral-300"
      >
        Cancel
      </button>
    </div>
  );
}
