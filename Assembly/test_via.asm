VIA     = $6000
PORTB   = VIA
PORTA   = VIA + 1
DDRB    = VIA + 2
DDRA    = VIA + 3
T1C_L   = VIA + 4
T1C_H   = VIA + 5
T1L_L   = VIA + 6
T1L_H   = VIA + 7
T2C_L   = VIA + 8
T2C_H   = VIA + 9
SR      = VIA + 10
ACR     = VIA + 11
PCR     = VIA + 12
IFR     = VIA + 13
IER     = VIA + 14
PA_NO_HS= VIA + 15

LED_RS = %10000000
LED_RW = %01000000
LED_E  = %00100000

msg_ptr         = $0000 ;2 bytes
saved_ddrb      = $0002 ;1 byte

        .org $8000

;Prepare LCD screen when it first is powered up
;Destroys: -
lcd_init:
        lda #%00110011  ;Set 8-bit mode twice
        jsr lcd_cmd

        lda #%00110010  ;Set 8-bit mode then 4-bit mode
        jsr lcd_cmd

        lda #%00101000  ;Function set: 4-bit mode, 2 lines, 5x8 pixels
        jsr lcd_cmd

        lda #%00000001  ;Screen clear
        jsr lcd_cmd

        lda #%00001110  ;Display switch: display ON, cursor ON, blink OFF
        jsr lcd_cmd

        rts

;Send the contents of A as a command to the LCD
;Destroys: -
lcd_cmd:
        phx             ;Save X contents on stack
        tax             ;Save command into X
        
        lsr
        lsr
        lsr
        lsr             ;Shift the 4 high-order bits into the low 4 bits

        sta PORTB       ;Send first half of the command

        ora #LED_E
        sta PORTB
        and #~LED_E
        sta PORTB       ;Send pulse on the Enable pin of the LCD

        txa             ;Retrieve saved command
        and #%00001111  ;Keep the 4 low-order bits
        
        sta PORTB       ;Send second half of the command

        ora #LED_E
        sta PORTB
        and #~LED_E
        sta PORTB       ;Send pulse on the Enable pin of the LCD

        plx             ;Retrieve X contents from stack
        rts

;Send the contents of A as data to the LCD
;Destroys: -
lcd_data:
        phx             ;Save X contents on stack
        tax             ;Save command into X
        
        lsr
        lsr
        lsr
        lsr             ;Shift the 4 high-order bits into the low 4 bits

        ora #LED_RS
        sta PORTB       ;Send first half of the command

        ora #LED_E
        sta PORTB
        and #~LED_E
        sta PORTB       ;Send pulse on the Enable pin of the LCD

        txa             ;Retrieve saved command
        and #%00001111  ;Keep the 4 low-order bits
        
        ora #LED_RS
        sta PORTB       ;Send second half of the command

        ora #LED_E
        sta PORTB
        and #~LED_E
        sta PORTB       ;Send pulse on the Enable pin of the LCD

        plx             ;Retrieve X contents from stack
        rts

;Prints the contents of number_str on the LCD
;Destroys: Y
lcd_print:
        ldy #0
lcd_print_nextchar:
        lda (msg_ptr), Y;Follow the pointer to the message
        beq lcd_print_end
        jsr lcd_data    ;Print char
        
        iny             ;Set index to next char
        jmp lcd_print_nextchar

lcd_print_end:
        rts

;Prints the contents of A as an 8-digit binary number on the LCD
;Destroys: X, Y
print_bin:
        rol
        rol
        ldy #8
print_bin_loop:
        tax
        
        ;Print rightmost bit
        and #%00000001
        ora #%00110000
        php
        jsr lcd_data
        plp

        txa
        rol

        dey
        bne print_bin_loop

        rts

true_DDRB_str:  .string "TRUE DDRB:"

PORTB_str:      .string "PORTB:"
PORTA_str:      .string "PORTA:"
DDRB_str:       .string "DDRB:"
DDRA_str:       .string "DDRA:"
T1C_L_str:      .string "T1C_L:"
T1C_H_str:      .string "T1C_H:"
T1L_L_str:      .string "T1L_L:"
T1L_H_str:      .string "T1L_H:"
T2C_L_str:      .string "T2C_L:"
T2C_H_str:      .string "T2C_H:"
SR_str:         .string "SR:"
ACR_str:        .string "ACR:"
PCR_str:        .string "PCR:"
IFR_str:        .string "IFR:"
IER_str:        .string "IER:"
PA_NO_HS_str:   .string "PA_NO_HS:"

str_table:
        .word PORTB_str
        .word PORTA_str
        .word DDRB_str
        .word DDRA_str
        .word T1C_L_str
        .word T1C_H_str
        .word T1L_L_str
        .word T1L_H_str
        .word T2C_L_str
        .word T2C_H_str
        .word SR_str
        .word ACR_str
        .word PCR_str
        .word IFR_str
        .word IER_str
        .word PA_NO_HS_str

;Main program
irq:
nmi:
reset:  ldx #$ff
        txs             ;Initialize stack pointer to address 01ff

        lda DDRB
        sta saved_ddrb

        lda #%11101111  ;Set output B pins 
        sta DDRB        ;Initialize Port B to one input and full output

        jsr lcd_init    ;Set up LCD screen

        lda #%00000110  ;Entry set: increment, shift cursor
        jsr lcd_cmd

        ;Print true DDRB string
        lda #<true_DDRB_str
        sta msg_ptr
        lda #>true_DDRB_str
        sta msg_ptr + 1
        jsr lcd_print

        ;Print bin number on the 2nd line
        lda #$80 | $40  ;Set DDRAM address
        jsr lcd_cmd
        lda saved_ddrb
        jsr print_bin

        ldx #0
next_addr:
        ;Put VIA register name string pointer in msg_ptr
        lda str_table, X
        sta msg_ptr
        inx
        lda str_table, X
        sta msg_ptr + 1

        ;Print VIA register name on 1st line
        lda #%00000001  ;Screen clear
        jsr lcd_cmd
        jsr lcd_print

        ;Move cursor to 2nd line
        lda #$80 | $40  ;Set DDRAM address
        jsr lcd_cmd

        ;Divide X by two for indexing
        txa
        lsr
        tax
        rol             ;Get true X value back
        pha             ;Save true X value on the stack

        ;Print VIA register value
        lda PORTB, X
        jsr print_bin
        plx             ;Retrieve X from the stack

        inx
        cpx #32
        bne next_addr

        stp

        .blk 3, $ea

        .org $fffa
        .word nmi       ;NMI vector
        .word reset     ;Reset vector
        .word irq       ;IRQ vector