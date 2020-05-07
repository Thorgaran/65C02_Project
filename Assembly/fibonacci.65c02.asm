VIA     = $6000
PORTB   = VIA
PORTA   = VIA + 1
DDRB    = VIA + 2
DDRA    = VIA + 3

PREV_VAL= $00

        .org $8000

reset:  ldx #$ff
        txs             ;Initialize stack pointer to address 01ff
        stx DDRB        ;Initialize Port B to full output

reset_fib:
        stz PREV_VAL    ;Initialize PREV_VAL to 0
        lda #$01        ;Initialize A to 1
        sta PORTB       ;Print A
        clc             ;Clear carry

loop:   tax             ;Transfer n to not lose it
        adc PREV_VAL    ;Get n+1 = n + n-1
        bcs reset_fib   ;If n+1 > 255, reset
        stx PREV_VAL    ;Store n
        sta PORTB       ;Print n+1
        jmp loop        ;Do next iteration

nmi:
irq:    lda #$ff        ;Error routine (if a BRK happened)
        sta PORTB       ;Print 11111111
        stz PORTB       ;Print 00000000
        jmp irq         ;Loop error

        .blk 3, $ea

        .org $fffa
        .word nmi       ;NMI vector
        .word reset     ;Reset vector
        .word irq       ;IRQ vector