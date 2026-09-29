// Base structure
// Profile picture dropdown menu
function toggleDropdown() {
    const menu = document.getElementById("dropdown-menu");
    menu.classList.toggle("show");
}
window.onclick = function(event) {
    if (!event.target.matches('.profilepic')) {
    const dropdowns = document.getElementsByClassName("dropdown-content");
    for (let i = 0; i < dropdowns.length; i++) {
        let openDropdown = dropdowns[i];
        if (openDropdown.classList.contains('show')) {
        openDropdown.classList.remove('show');
        }
    }
    }
}
// Profile picture dropdown menu end
// Base structure


