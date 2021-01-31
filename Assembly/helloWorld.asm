SAVE_X  = $0000

VIA     = $6000
PORTB   = VIA
PORTA   = VIA + 1
DDRB    = VIA + 2
DDRA    = VIA + 3

LED_RS = %10000000
LED_RW = %01000000
LED_E  = %00100000

        .org $8000

lcd_init:
        lda #%00110011  ;Set 8-bit mode twice
        jsr lcd_cmd

        lda #%00110010  ;Set 8-bit mode then 4-bit mode
        jsr lcd_cmd

        lda #%00101000  ;Function set: 4-bit mode, 2 lines, 5x8 pixels
        jsr lcd_cmd

        lda #%00000001  ;Screen clear
        jsr lcd_cmd

        lda #%00001111  ;Display switch: display ON, cursor ON, blink ON
        jsr lcd_cmd

        lda #%00000110  ;Entry set: increment, shift cursor
        jsr lcd_cmd

        rts

lcd_send_1:
        stx SAVE_X      ;Save X contents in RAM
        tax             ;Save command into X
        
        lsr
        lsr
        lsr
        lsr             ;Shift the 4 high-order bits into the low 4 bits

        rts

lcd_send_2:
        sta PORTB       ;Send first half of the command

        ora #LED_E
        sta PORTB
        and #~LED_E
        sta PORTB       ;Send pulse on the Enable pin of the LCD

        txa             ;Retrieve saved command
        and #%00001111  ;Keep the 4 low-order bits

        rts

lcd_send_3:
        sta PORTB       ;Send second half of the command

        ora #LED_E
        sta PORTB
        and #~LED_E
        sta PORTB       ;Send pulse on the Enable pin of the LCD

        ldx SAVE_X      ;Retrieve X contents from RAM

        rts

lcd_cmd:
        jsr lcd_send_1
        jsr lcd_send_2
        jsr lcd_send_3

        rts

lcd_data:
        jsr lcd_send_1
        ora #LED_RS
        jsr lcd_send_2
        ora #LED_RS
        jsr lcd_send_3

        rts
        
irq:
nmi:
reset:  ldx #$ff
        txs             ;Initialize stack pointer to address 01ff
        
        lda #%11101111  ;Set output B pins 
        sta DDRB        ;Initialize Port B to one input and #full output

        jsr lcd_init    ;Set up LCD screen

        lda #"H"
        jsr lcd_data
        lda #"e"
        jsr lcd_data
        lda #"l"
        jsr lcd_data
        lda #"l"
        jsr lcd_data
        lda #"o"
        jsr lcd_data
        lda #","
        jsr lcd_data
        lda #" "
        jsr lcd_data
        lda #"W"
        jsr lcd_data
        lda #"o"
        jsr lcd_data
        lda #"r"
        jsr lcd_data
        lda #"l"
        jsr lcd_data
        lda #"d"
        jsr lcd_data
        lda #"!"
        jsr lcd_data

        stp

        .blk 3, $ea

        .org $fffa
        .word nmi       ;NMI vector
        .word reset     ;Reset vector
        .word irq       ;IRQ vector
        