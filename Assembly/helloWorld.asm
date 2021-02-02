MSG_PTR = $00

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

lcd_print:
        phy             ;Save Y contents on stack
        ldy #0
lcd_print_line1:
        lda (MSG_PTR), Y;Follow the pointer to the message
        beq lcd_print_next
        jsr lcd_data    ;Print char
        iny             ;Set index to next char
        jmp lcd_print_line1
lcd_print_next:
        iny             ;Set index to next char
        lda (MSG_PTR), Y;Read first char of 2nd line
        beq lcd_end     ;If the 2nd line is empty, shortcut to the end
        
        lda #($80|$40)  ;Set DDRAM address
        jsr lcd_cmd     ;Send cursor to 2nd line
lcd_print_line2:
        lda (MSG_PTR), Y;Follow the pointer to the message
        beq lcd_end
        jsr lcd_data    ;Print char
        iny             ;Set index to next char
        jmp lcd_print_line2
lcd_end:
        ply             ;Retrieve Y contents from stack
        rts
        
irq:
nmi:
reset:  ldx #$ff
        txs             ;Initialize stack pointer to address 01ff
        
        lda #%11101111  ;Set output B pins 
        sta DDRB        ;Initialize Port B to one input and full output

        jsr lcd_init    ;Set up LCD screen

        lda #%11111111  ;Set output A pins 
        sta DDRA        ;Initialize Port A to full output
        sta PORTA       ;Light all A pins for debug

        lda #<msg
        sta MSG_PTR
        lda #>msg
        sta MSG_PTR + 1
        jsr lcd_print

        stp

msg:    .string "Hello, world!"
        .string ""

        .blk 3, $ea

        .org $fffa
        .word nmi       ;NMI vector
        .word reset     ;Reset vector
        .word irq       ;IRQ vector
        