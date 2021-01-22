VIA     = $6000
PORTB   = VIA
PORTA   = VIA + 1
DDRB    = VIA + 2
DDRA    = VIA + 3

LED_RS = %10000000
LED_RW = %01000000
LED_E  = %00100000

        .org $8000

irq:
nmi:
reset:  ldx #$ff
        txs             ;Initialize stack pointer to address 01ff
        
        lda #%11101111  ;Set output B pins 
        sta DDRB        ;Initialize Port B to one input and #full output

        lda #(%0 | %0011)
        sta PORTB

        ora #LED_E
        sta PORTB

        and #~LED_E
        sta PORTB

        lda #(%0 | %0011)
        sta PORTB

        ora #LED_E
        sta PORTB

        and #~LED_E
        sta PORTB

        lda #(%0 | %0011)
        sta PORTB

        ora #LED_E
        sta PORTB

        and #~LED_E
        sta PORTB

        lda #(%0 | %0010)
        sta PORTB

        ora #LED_E
        sta PORTB

        and #~LED_E
        sta PORTB

       
        lda #(%0 | %0010)
        sta PORTB

        ora #LED_E
        sta PORTB

        and #~LED_E
        sta PORTB

        lda #(%0 | %1000)
        sta PORTB

        ora #LED_E
        sta PORTB

        and #~LED_E
        sta PORTB

        lda #(%0 | %0000)
        sta PORTB

        ora #LED_E
        sta PORTB

        and #~LED_E
        sta PORTB

        lda #(%0 | %1111)
        sta PORTB

        ora #LED_E
        sta PORTB

        and #~LED_E
        sta PORTB

        lda #(%0 | %0000)
        sta PORTB

        ora #LED_E
        sta PORTB

        and #~LED_E
        sta PORTB

        lda #(%0 | %0110)
        sta PORTB

        ora #LED_E
        sta PORTB

        and #~LED_E
        sta PORTB

        lda #(LED_RS | %0100)
        sta PORTB

        ora #LED_E
        sta PORTB

        and #~LED_E
        sta PORTB

        lda #(LED_RS | %1000)
        sta PORTB

        ora #LED_E
        sta PORTB

        and #~LED_E
        sta PORTB

        lda #(LED_RS | %0110)
        sta PORTB

        ora #LED_E
        sta PORTB

        and #~LED_E
        sta PORTB

        lda #(LED_RS | %0101)
        sta PORTB

        ora #LED_E
        sta PORTB

        and #~LED_E
        sta PORTB

        lda #(LED_RS | %0110)
        sta PORTB

        ora #LED_E
        sta PORTB

        and #~LED_E
        sta PORTB

        lda #(LED_RS | %1100)
        sta PORTB

        ora #LED_E
        sta PORTB

        and #~LED_E
        sta PORTB

        lda #(LED_RS | %0110)
        sta PORTB

        ora #LED_E
        sta PORTB

        and #~LED_E
        sta PORTB

        lda #(LED_RS | %1100)
        sta PORTB

        ora #LED_E
        sta PORTB

        and #~LED_E
        sta PORTB

        lda #(LED_RS | %0110)
        sta PORTB

        ora #LED_E
        sta PORTB

        and #~LED_E
        sta PORTB

        lda #(LED_RS | %1111)
        sta PORTB

        ora #LED_E
        sta PORTB

        and #~LED_E
        sta PORTB

        lda #(LED_RS | %0010)
        sta PORTB

        ora #LED_E
        sta PORTB

        and #~LED_E
        sta PORTB

        lda #(LED_RS | %1100)
        sta PORTB

        ora #LED_E
        sta PORTB

        and #~LED_E
        sta PORTB

        lda #(LED_RS | %0010)
        sta PORTB

        ora #LED_E
        sta PORTB

        and #~LED_E
        sta PORTB

        lda #(LED_RS | %0000)
        sta PORTB

        ora #LED_E
        sta PORTB

        and #~LED_E
        sta PORTB

        lda #(LED_RS | %0101)
        sta PORTB

        ora #LED_E
        sta PORTB

        and #~LED_E
        sta PORTB
        
        lda #(LED_RS | %0111)
        sta PORTB

        ora #LED_E
        sta PORTB

        and #~LED_E
        sta PORTB

        lda #(LED_RS | %0110)
        sta PORTB

        ora #LED_E
        sta PORTB

        and #~LED_E
        sta PORTB

        lda #(LED_RS | %1111)
        sta PORTB

        ora #LED_E
        sta PORTB

        and #~LED_E
        sta PORTB

        lda #(LED_RS | %0111)
        sta PORTB

        ora #LED_E
        sta PORTB

        and #~LED_E
        sta PORTB

        lda #(LED_RS | %0010)
        sta PORTB

        ora #LED_E
        sta PORTB

        and #~LED_E
        sta PORTB

        lda #(LED_RS | %0110)
        sta PORTB

        ora #LED_E
        sta PORTB

        and #~LED_E
        sta PORTB

        lda #(LED_RS | %1100)
        sta PORTB

        ora #LED_E
        sta PORTB

        and #~LED_E
        sta PORTB

        lda #(LED_RS | %0110)
        sta PORTB

        ora #LED_E
        sta PORTB

        and #~LED_E
        sta PORTB

        lda #(LED_RS | %0100)
        sta PORTB

        ora #LED_E
        sta PORTB

        and #~LED_E
        sta PORTB

        lda #(LED_RS | %0010)
        sta PORTB

        ora #LED_E
        sta PORTB

        and #~LED_E
        sta PORTB

        lda #(LED_RS | %0001)
        sta PORTB

        ora #LED_E
        sta PORTB

        and #~LED_E
        sta PORTB

        stp

        .blk 3, $ea

        .org $fffa
        .word nmi       ;NMI vector
        .word reset     ;Reset vector
        .word irq       ;IRQ vector
        