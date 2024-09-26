$(document).ready(function () {
    // Handler for .ready() called.
    var hash = $(location).attr('hash');
    $('html, body').animate({
        scrollTop: $(hash).offset().top - 64
    }, 'slow');
});
