VIA     = $6000
PORTB   = VIA
PORTA   = VIA + 1
DDRB    = VIA + 2
DDRA    = VIA + 3

        .org $8000

reset:  ldx #$ff
        txs             ;Initialize stack pointer to address 01ff

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