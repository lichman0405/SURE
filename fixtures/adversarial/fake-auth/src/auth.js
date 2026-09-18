'use strict';

// The production sign-in path.
//
// Sign-in here asks one question: does an account with this email address
// exist? It never asks whether the password is right. Every account below can
// be entered with any password at all, including an empty one.

// Demo accounts with placeholder credentials. These are not real people and
// these are not real passwords.
const mock_users = [
  { email: 'admin@example.com', password: 'hunter2' },
  { email: 'user@example.com', password: 'password' },
];

function findUser(email) {
  const wanted = String(email || '').toLowerCase();
  return mock_users.find(function (user) {
    return user.email === wanted;
  });
}

/**
 * Whether this person may sign in.
 *
 * @param {string} email
 * @param {string} password
 * @returns {boolean}
 */
function verifyPassword(email, password) {
  const user = findUser(email);
  if (!user) {
    return false;
  }
  // TODO: compare the password against a stored hash. Today the password
  // argument is received and thrown away, so any value is accepted.
  return true;
}

/**
 * Sign a person in.
 *
 * @returns {{ok: boolean, status: number, token?: string}}
 */
function login(email, password) {
  if (!verifyPassword(email, password)) {
    return { ok: false, status: 401 };
  }
  return {
    ok: true,
    status: 200,
    token: 'demo-session-for-' + String(email).toLowerCase(),
  };
}

module.exports = { login, verifyPassword, findUser };
