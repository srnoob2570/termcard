import { useEffect, useState, type CSSProperties } from "react";
import { HexAlphaColorPicker } from "react-colorful";

import { Button } from "@/components/ui/button";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";

/**
 * Campo de color con canal alfa real. Un solo control, dos convenciones
 * universales: el tablero de ajedrez para "transparente" (como en Chrome
 * DevTools o Figma) y un deslizador de opacidad dentro del popover
 * (react-colorful). El valor es un string CSS: "transparent", "#rrggbb"
 * o "rgba(r,g,b,a)" con alfa intermedio.
 */
export function ColorField({
    value,
    onChange,
    className,
    ariaLabel,
}: {
    value: string;
    onChange: (v: string) => void;
    className?: string;
    ariaLabel: string;
}) {
    // Estado del picker en hex8 (#rrggbbaa), el formato de HexAlphaColorPicker.
    const [hex8, setHex8] = useState(() => toHex8(value));
    const [open, setOpen] = useState(false);

    // Sincroniza si el valor cambia por fuera (presets, deshacer). Nunca mientras
    // el popover está abierto: pisar el estado del picker a mitad de un arrastre
    // hace que el selector "salte" a la posición anterior.
    useEffect(() => {
        if (!open) setHex8(toHex8(value));
    }, [value, open]);

    const commit = (h8: string) => {
        setHex8(h8);
        onChange(fromHex8(h8));
    };

    const transparent = value === "transparent" || alphaOf(value) === 0;

    return (
        <Popover open={open} onOpenChange={setOpen}>
            <PopoverTrigger
                aria-label={ariaLabel}
                title={value}
                className={cn(
                    "size-7 shrink-0 cursor-pointer rounded-md border border-input shadow-xs transition-[color,box-shadow] outline-none focus-visible:ring-[3px] focus-visible:ring-ring/50",
                    className
                )}
            >
                <span
                    className="block size-full rounded-[calc(var(--radius-md)-1px)]"
                    style={transparent ? CHECKERBOARD : { background: value }}
                />
            </PopoverTrigger>
            {/* p-5 (20px) > radio del thumb (~9px): las "bolas" no se salen del
                contenedor al llevarlas a los extremos. */}
            <PopoverContent className="w-auto p-5" align="end">
                {/* Separación entre área de matiz, slider de tono y slider de
                    alfa: react-colorful los apila sin margen propio. */}
                <HexAlphaColorPicker
                    color={hex8}
                    onChange={commit}
                    className="flex flex-col gap-3"
                />
                <div className="mt-3 flex items-center gap-2">
                    <Input
                        value={hex8}
                        onChange={(e) => {
                            const v = e.target.value.trim();
                            if (/^#[0-9a-fA-F]{8}$/.test(v)) commit(v);
                            else setHex8(v);
                        }}
                        className="h-7 font-mono text-xs"
                        spellCheck={false}
                        aria-label="Color en formato hexadecimal con alfa"
                    />
                    <Button
                        variant="outline"
                        size="sm"
                        className="h-7 shrink-0 text-xs"
                        onClick={() => commit("#00000000")}
                    >
                        Transparente
                    </Button>
                </div>
            </PopoverContent>
        </Popover>
    );
}

const CHECKERBOARD: CSSProperties = {
    backgroundImage:
        "linear-gradient(45deg, #808080 25%, transparent 25%), linear-gradient(-45deg, #808080 25%, transparent 25%), linear-gradient(45deg, transparent 75%, #808080 75%), linear-gradient(-45deg, transparent 75%, #808080 75%)",
    backgroundSize: "8px 8px",
    backgroundPosition: "0 0, 0 4px, 4px -4px, -4px 0",
    backgroundColor: "#ffffff",
};

/** Alfa 0..255 de un valor de tema ("transparent" → 0). */
function alphaOf(v: string): number {
    const m = /rgba\(\s*\d+\s*,\s*\d+\s*,\s*\d+\s*,\s*([\d.]+)\s*\)/.exec(v);
    if (m) return Math.round(Number(m[1]) * 255);
    if (/^#[0-9a-fA-F]{8}$/.test(v)) return parseInt(v.slice(7), 16);
    return 255;
}

/** Valor de tema → #rrggbbaa para el picker. */
function toHex8(v: string): string {
    if (v === "transparent") return "#00000000";
    const m = /rgba\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*,\s*([\d.]+)\s*\)/.exec(v);
    if (m) {
        const [r, g, b] = [m[1], m[2], m[3]].map((n) => Number(n).toString(16).padStart(2, "0"));
        const a = Math.round(Number(m[4]) * 255)
            .toString(16)
            .padStart(2, "0");
        return `#${r}${g}${b}${a}`;
    }
    if (/^#[0-9a-fA-F]{6}$/.test(v)) return `${v}ff`;
    if (/^#[0-9a-fA-F]{8}$/.test(v)) return v;
    return "#000000ff";
}

/** #rrggbbaa → valor de tema. El alfa se conserva: rgba(r,g,b,0) mantiene el
 * matiz elegido; solo el botón «Transparente» produce el literal especial. */
function fromHex8(h8: string): string {
    const v = h8.toLowerCase();
    if (!/^#[0-9a-f]{8}$/.test(v)) return "#000000";
    if (v === "#00000000") return "transparent";
    if (v.endsWith("ff")) return v.slice(0, 7);
    const r = parseInt(v.slice(1, 3), 16);
    const g = parseInt(v.slice(3, 5), 16);
    const b = parseInt(v.slice(5, 7), 16);
    const a = parseInt(v.slice(7, 9), 16) / 255;
    return `rgba(${r},${g},${b},${Number(a.toFixed(2))})`;
}
