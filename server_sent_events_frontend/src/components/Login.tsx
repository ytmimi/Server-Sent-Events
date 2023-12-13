import { useRef, useState } from "react";
import { validate as uuidValidate } from "uuid";

type LoginProps = {
  userId: string;
  setUserId: (arg0: string) => void;
};

export function Login({ userId, setUserId }: LoginProps) {
  const [hasErrors, setHasErrors] = useState(false);
  const ref = useRef(userId);

  let errorMessage = "";

  if (hasErrors) {
    errorMessage = "User ID must be a UUID";
  }

  return (
    <>
      <h1>Login</h1>
      <p style={{ color: "red" }}>{errorMessage}</p>
      <label>User ID (UUID): </label>
      <input
        name="userIdInput"
        onChange={(e) => (ref.current = e.target.value)}
      />
      <button
        onClick={() => {
          if (uuidValidate(ref.current)) {
            setUserId(ref.current);
          } else {
            setHasErrors(true);
          }
        }}
      >
        Login
      </button>
    </>
  );
}
