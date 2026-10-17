import { Component, type ReactNode } from 'react';

interface Props {
  fallback: ReactNode;
  /** A new value clears a failure, so the children render again. */
  resetKey?: string;
  children: ReactNode;
}

interface State {
  failed: boolean;
  resetKey?: string;
}

/** Shows `fallback` when a child throws during render, such as a rejected `use()` promise. */
export class ErrorBoundary extends Component<Props, State> {
  state: State = { failed: false, resetKey: this.props.resetKey };

  static getDerivedStateFromError(): Partial<State> {
    return { failed: true };
  }

  static getDerivedStateFromProps(props: Props, state: State): Partial<State> | null {
    return props.resetKey === state.resetKey ? null : { failed: false, resetKey: props.resetKey };
  }

  componentDidCatch(error: unknown) {
    console.error(error);
  }

  render() {
    return this.state.failed ? this.props.fallback : this.props.children;
  }
}
